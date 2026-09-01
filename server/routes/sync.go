package routes

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"llava-server/middleware"
	"llava-server/models"
	"strconv"
	"time"

	"strings"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/service/s3"
	"github.com/aws/smithy-go"
	"github.com/gofiber/fiber/v3"
	"github.com/gofiber/fiber/v3/log"
	"github.com/google/uuid"
	"go.mongodb.org/mongo-driver/v2/bson"
	"go.mongodb.org/mongo-driver/v2/mongo"
	"go.mongodb.org/mongo-driver/v2/mongo/options"
	"golang.org/x/sync/errgroup"
)

var ErrQuotaExceeded = errors.New("quota exceeded")

type ReservationStatus string

const (
	ReservationPending  ReservationStatus = "pending"
	ReservationConsumed ReservationStatus = "consumed"
)

func (s *SyncHandler) SyncCheck(c fiber.Ctx) error {
	userID := c.Locals("userID").(string)

	syncCheckRequest := new(models.CheckSyncRequest)

	if err := c.Bind().Body(syncCheckRequest); err != nil {
		log.Errorw("sync check: failed to parse body", "user_id", userID, "error", err)
		return middleware.BadRequest("Couldnt read body data")
	}

	if err := s.Validator.Struct(syncCheckRequest); err != nil {
		log.Errorw("sync check: validation failed", "user_id", userID, "error", err)
		return middleware.BadRequest("Wrong check sync struct was sent")
	}

	ctx, cancel := context.WithTimeout(
		context.Background(),
		50*time.Second,
	)
	defer cancel()

	notesToCheck := syncCheckRequest.Notes

	log.Debugw(
		"sync check started",
		"user_id", userID,
		"notes_count", len(notesToCheck),
		"full_sync", syncCheckRequest.FullSync,
	)

	notesToUpload := []string{}
	notesToDownload := []models.DownloadNote{}
	notesToHardDelete := []string{}
	syncedNotes := []string{}

	attachmentsToUpload := []models.UploadAttachment{}
	attachmentsToDownload := []models.DownloadAttachment{}
	syncedAttachments := []string{}

	notesFailed := []string{}
	attachmentsToHardDelete := []string{}
	quotaExceeded := false
	g, ctx := errgroup.WithContext(ctx)

	results := make(chan NoteSyncResult, len(notesToCheck))

	if syncCheckRequest.FullSync {
		arr, attArr, err := checkNotesNotExistingOnClient(
			s,
			ctx,
			userID,
			syncCheckRequest.Notes,
		)
		// TODO add sync on events like singed in onlin etc, plus interval 30 seconds + ui bottom bar
		if err != nil {
			log.Debugw("Couldnt get notes that are on server but not on the client", "user_id", userID, "error", err)
		} else {
			notesToDownload = arr
			attachmentsToDownload = append(attachmentsToDownload, attArr...)

			log.Debugw(
				"full sync scan completed",
				"user_id", userID,
				"notes_missing_on_client", len(arr),
				"attachments_missing_on_client", len(attArr),
			)
		}
	}

	for _, note := range notesToCheck {
		note := note

		g.Go(func() error {
			result, err := checkNoteSync(
				note,
				ctx,
				userID,
				s,
			)

			if err != nil {
				log.Errorw(
					"note sync failed, skipping note",
					"local_id", note.LocalID,
					"error", err,
				)

				results <- NoteSyncResult{
					FailedNotes: []string{note.LocalID},
				}

				return nil
			}

			results <- result
			return nil
		})
	}

	if err := g.Wait(); err != nil {
		log.Errorw("sync check: errgroup failed", "user_id", userID, "error", err)
		return middleware.Internal(err.Error())
	}

	close(results)

	for result := range results {
		if result.quotaExceeded {
			quotaExceeded = true
		}
		notesToUpload = append(
			notesToUpload,
			result.NotesToUpload...,
		)

		notesToDownload = append(
			notesToDownload,
			result.NotesToDownload...,
		)

		notesToHardDelete = append(
			notesToHardDelete,
			result.NotesToHardDelete...,
		)

		syncedNotes = append(
			syncedNotes,
			result.SyncedNotes...,
		)

		attachmentsToUpload = append(
			attachmentsToUpload,
			result.AttachmentsToUpload...,
		)

		attachmentsToDownload = append(
			attachmentsToDownload,
			result.AttachmentsToDownload...,
		)

		syncedAttachments = append(
			syncedAttachments,
			result.SyncedAttachments...,
		)

		notesFailed = append(
			notesFailed,
			result.FailedNotes...,
		)

		attachmentsToHardDelete = append(
			attachmentsToHardDelete,
			result.AttachmentsToHardDelete...,
		)

	}

	log.Debugw(
		"sync check completed",
		"user_id", userID,
		"notes_to_upload", notesToUpload,
		"notes_to_download_count", len(notesToDownload),
		"notes_synced_count", len(syncedNotes),
		"notes_to_hard_delete_count", len(notesToHardDelete),
		"notes_failed", notesFailed,
		"attachments_to_upload_count", len(attachmentsToUpload),
		"attachments_to_download_count", len(attachmentsToDownload),
		"attachments_synced_count", len(syncedAttachments),
		"attachments_to_hard_delete_count", len(attachmentsToHardDelete),
	)

	response := models.CheckSyncResponse{
		ToUpload:                notesToUpload,
		NotesToDownload:         notesToDownload,
		NotesSynced:             syncedNotes,
		NotesToHardDelete:       notesToHardDelete,
		AttachmentsToUpload:     attachmentsToUpload,
		AttachmentsSynced:       syncedAttachments,
		AttachmentsToDownload:   attachmentsToDownload,
		NotesFailed:             notesFailed,
		AttachmentsToHardDelete: attachmentsToHardDelete,
		QuotaExceeded:           quotaExceeded,
	}

	return c.Status(200).JSON(response)
}

type NoteSyncResult struct {
	NotesToUpload           []string
	NotesToDownload         []models.DownloadNote
	NotesToHardDelete       []string
	SyncedNotes             []string
	AttachmentsToUpload     []models.UploadAttachment
	AttachmentsToDownload   []models.DownloadAttachment
	SyncedAttachments       []string
	FailedNotes             []string
	AttachmentsToHardDelete []string
	quotaExceeded           bool
}

func checkNoteSync(
	note models.CheckNoteSyncStatus,
	ctx context.Context,
	userID string,
	s *SyncHandler,
) (NoteSyncResult, error) {
	result := NoteSyncResult{
		NotesToUpload:         []string{},
		NotesToDownload:       []models.DownloadNote{},
		NotesToHardDelete:     []string{},
		SyncedNotes:           []string{},
		AttachmentsToUpload:   []models.UploadAttachment{},
		AttachmentsToDownload: []models.DownloadAttachment{},
		SyncedAttachments:     []string{},
		quotaExceeded:         false,
	}

	cloudID := note.CloudID

	cloudIDStr := "<none>"
	if cloudID != nil {
		cloudIDStr = cloudID.Hex()
	}

	log.Debugw(
		"checking note sync status",
		"local_id", note.LocalID,
		"cloud_id", cloudIDStr,
		"hard_deleted", note.HardDeleted,
	)

	if note.HardDeleted {
		if cloudID == nil {
			// **The note never existed in the cloud.**
			log.Debugw(
				"note hard deleted locally and never existed in cloud, nothing to do",
				"local_id", note.LocalID,
			)
			return result, nil
		}

		if note.CloudVersion == nil {
			return NoteSyncResult{}, fmt.Errorf(
				"cloud version is required when hard deleting note %s",
				note.LocalID,
			)
		}

		if err := handleClientHardDelete(
			note,
			ctx,
			userID,
			s,
		); err != nil {
			return NoteSyncResult{}, fmt.Errorf(
				"hard delete note %s: %w",
				note.LocalID,
				err,
			)
		}

		// **The client already deleted the note locally.**
		result.SyncedNotes = append(
			result.SyncedNotes,
			note.LocalID,
		)

		return result, nil
	}

	if cloudID == nil {
		log.Debugw(
			"note has no cloud id yet, marking for upload",
			"local_id", note.LocalID,
		)

		result.NotesToUpload = append(
			result.NotesToUpload,
			note.LocalID,
		)

		return result, nil
	}

	if note.CloudVersion == nil {
		return NoteSyncResult{}, fmt.Errorf(
			"cloud version is required when cloud ID is set for note %s",
			note.LocalID,
		)
	}

	foundNote := new(models.DownloadNote)

	if err := s.acquireWorker(ctx); err != nil {
		return NoteSyncResult{}, err
	}

	err := s.Coll.FindOne(
		ctx,
		bson.M{
			"_id":      cloudID,
			"owner_id": userID,
		},
	).Decode(foundNote)

	s.releaseWorker()

	if err != nil {
		if !errors.Is(err, mongo.ErrNoDocuments) {
			log.Errorw(
				"lookup note in cloud failed",
				"local_id", note.LocalID,
				"cloud_id", cloudIDStr,
				"user_id", userID,
				"error", err,
			)

			return NoteSyncResult{}, fmt.Errorf(
				"lookup note %s in cloud: %w",
				note.LocalID,
				err,
			)
		}

		log.Debugw(
			"note has cloud id but no matching document found, marking for upload",
			"local_id", note.LocalID,
			"cloud_id", cloudIDStr,
			"user_id", userID,
		)

		result.NotesToUpload = append(
			result.NotesToUpload,
			note.LocalID,
		)

		return result, nil
	}

	if foundNote.HardDeleted {
		log.Debugw(
			"cloud note is a tombstone, marking for hard delete on client",
			"local_id", note.LocalID,
			"cloud_id", cloudIDStr,
		)

		if err := cleanupDeletedNoteAttachments(
			*foundNote,
			ctx,
			userID,
			s,
		); err != nil {
			return NoteSyncResult{}, fmt.Errorf(
				"cleanup deleted note %s: %w",
				note.LocalID,
				err,
			)
		}

		result.NotesToHardDelete = append(
			result.NotesToHardDelete,
			note.LocalID,
		)

		return result, nil
	}

	if foundNote.CloudVersion > *note.CloudVersion {
		log.Debugw(
			"cloud version is newer, marking for download",
			"local_id", note.LocalID,
			"cloud_version", foundNote.CloudVersion,
			"client_version", *note.CloudVersion,
		)

		result.NotesToDownload = append(
			result.NotesToDownload,
			*foundNote,
		)
	} else if foundNote.CloudVersion < *note.CloudVersion {
		log.Debugw(
			"client version is newer, marking for upload",
			"local_id", note.LocalID,
			"cloud_version", foundNote.CloudVersion,
			"client_version", *note.CloudVersion,
		)

		result.NotesToUpload = append(
			result.NotesToUpload,
			note.LocalID,
		)
	} else {
		log.Debugw(
			"note versions match, already synced",
			"local_id", note.LocalID,
			"version", foundNote.CloudVersion,
		)

		result.SyncedNotes = append(
			result.SyncedNotes,
			note.LocalID,
		)
	}

	attachmentGroup, attachmentCtx := errgroup.WithContext(ctx)

	attachmentResults := make(chan struct {
		upload        []models.UploadAttachment
		download      []models.DownloadAttachment
		synced        []string
		hardDelete    []string
		quotaExceeded bool
	}, len(note.Attachments))

	for _, attachment := range note.Attachments {
		attachment := attachment

		attachmentGroup.Go(func() error {
			quotaExceeded,
				attachmentsToUpload,
				attachmentsToDownload,
				syncedAttachments,
				attachmentsToHardDelete,
				err := checkAttachment(
				attachment,
				userID,
				attachmentCtx,
				s,
				cloudID.Hex(),
			)
			if err != nil {
				return fmt.Errorf(
					"check attachment %s for note %s: %w",
					attachment.AttachmentID.String(),
					note.LocalID,
					err,
				)
			}

			attachmentResults <- struct {
				upload        []models.UploadAttachment
				download      []models.DownloadAttachment
				synced        []string
				hardDelete    []string
				quotaExceeded bool
			}{
				upload:        attachmentsToUpload,
				download:      attachmentsToDownload,
				synced:        syncedAttachments,
				hardDelete:    attachmentsToHardDelete,
				quotaExceeded: quotaExceeded,
			}

			return nil
		})
	}

	if err := attachmentGroup.Wait(); err != nil {
		close(attachmentResults)

		return NoteSyncResult{}, fmt.Errorf(
			"check attachments for note %s: %w",
			note.LocalID,
			err,
		)
	}

	close(attachmentResults)
	for attachmentResult := range attachmentResults {
		if attachmentResult.quotaExceeded {
			result.quotaExceeded = true
		}
		result.AttachmentsToUpload = append(
			result.AttachmentsToUpload,
			attachmentResult.upload...,
		)

		result.AttachmentsToDownload = append(
			result.AttachmentsToDownload,
			attachmentResult.download...,
		)

		result.SyncedAttachments = append(
			result.SyncedAttachments,
			attachmentResult.synced...,
		)

		result.AttachmentsToHardDelete = append(
			result.AttachmentsToHardDelete,
			attachmentResult.hardDelete...,
		)

	}

	return result, nil
}

// hardDeleteNoteInCloud performs the actual hard-delete of a note: writes a
// tombstone guarded by optimistic concurrency (the note must still be at
// clientCloudVersion and must not already be a tombstone), then deletes the
// given S3 attachment object keys. This is the single source of truth for
// hard-delete, shared by both entry points that can trigger it:
//   - handleClientHardDelete, from the sync-check flow (SyncCheck)
//   - UpdateNote, when the client sends hard_deleted=true on PUT /update-note
//
// Returns the resulting tombstone document (server-side cloud_version
// included) so callers can report it back to the client.
func hardDeleteNoteInCloud(
	ctx context.Context,
	s *SyncHandler,
	userID string,
	cloudID bson.ObjectID,
	clientCloudVersion int64,
	attachmentIDsToDelete []string,
) (*models.DownloadNote, error) {
	deletedAttachmentIDs := attachmentIDsToDelete
	if deletedAttachmentIDs == nil {
		deletedAttachmentIDs = []string{}
	}

	tombstone := models.DownloadNote{
		CloudID:            cloudID,
		OwnerID:            userID,
		CloudVersion:       clientCloudVersion + 1,
		HardDeleted:        true,
		DeletedAttachments: deletedAttachmentIDs,
	}

	if err := s.acquireWorker(ctx); err != nil {
		return nil, err
	}

	replaceResult, err := s.Coll.ReplaceOne(
		ctx,
		bson.M{
			"_id":           cloudID,
			"owner_id":      userID,
			"cloud_version": clientCloudVersion,
			"hard_deleted":  bson.M{"$ne": true},
		},
		tombstone,
	)

	s.releaseWorker()

	if err != nil {
		log.Errorw(
			"hard delete: store tombstone failed",
			"cloud_id", cloudID.Hex(),
			"owner_id", userID,
			"error", err,
		)

		return nil, fmt.Errorf(
			"store note tombstone: %w",
			err,
		)
	}

	if replaceResult.MatchedCount == 0 {
		// **The note may already have been hard deleted by another request,
		// may not exist at all, or the client's cloud_version was stale.**
		log.Debugw(
			"hard delete: ReplaceOne matched 0 documents, checking why",
			"cloud_id", cloudID.Hex(),
			"owner_id", userID,
			"client_cloud_version", clientCloudVersion,
		)

		foundNote := new(models.DownloadNote)

		if err := s.acquireWorker(ctx); err != nil {
			return nil, err
		}

		err := s.Coll.FindOne(
			ctx,
			bson.M{
				"_id":      cloudID,
				"owner_id": userID,
			},
		).Decode(foundNote)

		s.releaseWorker()

		if err != nil {
			if errors.Is(err, mongo.ErrNoDocuments) {
				log.Errorw(
					"hard delete: note does not exist in cloud at all",
					"cloud_id", cloudID.Hex(),
					"owner_id", userID,
				)

				return nil, fmt.Errorf(
					"note %s does not exist in cloud",
					cloudID.Hex(),
				)
			}

			log.Errorw(
				"hard delete: failed checking existing note tombstone",
				"cloud_id", cloudID.Hex(),
				"error", err,
			)

			return nil, fmt.Errorf(
				"check existing note tombstone: %w",
				err,
			)
		}

		if foundNote.HardDeleted {
			log.Debugw(
				"hard delete: note was already a tombstone, treating as success",
				"cloud_id", cloudID.Hex(),
			)

			return foundNote, nil
		}

		log.Errorw(
			"hard delete: note version mismatch, real server version differs from what client sent",
			"cloud_id", cloudID.Hex(),
			"client_cloud_version", clientCloudVersion,
			"actual_server_cloud_version", foundNote.CloudVersion,
		)

		return nil, fmt.Errorf(
			"note %s changed before hard delete",
			cloudID.Hex(),
		)
	}

	g, deleteCtx := errgroup.WithContext(ctx)

	for _, attachmentID := range deletedAttachmentIDs {
		attachmentID := attachmentID

		g.Go(func() error {
			cloudKey := createObjectKey(
				attachmentID,
				userID,
			)

			if err := deleteAttachmentObject(
				deleteCtx,
				s,
				cloudKey,
			); err != nil {
				return fmt.Errorf(
					"delete attachment %s: %w",
					attachmentID,
					err,
				)
			}

			return nil
		})
	}

	if err := g.Wait(); err != nil {
		return nil, fmt.Errorf(
			"delete S3 attachments: %w",
			err,
		)
	}

	tombstone.CloudVersion = clientCloudVersion + 1

	return &tombstone, nil
}

// handleClientHardDelete is the sync-check entry point into hard delete.
// It derives the attachment IDs to purge from note.Attachments, which the
// client is expected to supply as part of the sync-check payload for this
// note.
func handleClientHardDelete(
	note models.CheckNoteSyncStatus,
	ctx context.Context,
	userID string,
	s *SyncHandler,
) error {
	if note.CloudID == nil {
		return nil
	}

	if note.CloudVersion == nil {
		return errors.New("cloud version is required for hard delete")
	}

	deletedAttachmentIDs := make(
		[]string,
		0,
		len(note.Attachments),
	)

	for _, attachment := range note.Attachments {
		deletedAttachmentIDs = append(
			deletedAttachmentIDs,
			attachment.AttachmentID.String(),
		)
	}

	_, err := hardDeleteNoteInCloud(
		ctx,
		s,
		userID,
		*note.CloudID,
		*note.CloudVersion,
		deletedAttachmentIDs,
	)

	if err != nil {
		return err
	}

	return nil
}

// listAttachmentIDsForNote finds every attachment belonging to a note by
// scanning S3 and filtering on the note_cloud_id metadata each attachment
// object carries (the same technique already used in
// checkNotesNotExistingOnClient). Used by UpdateNote's hard-delete path,
// since NoteUploadRequest - unlike CheckNoteSyncStatus - has no attachments
// field for the client to supply IDs directly.
func listAttachmentIDsForNote(
	ctx context.Context,
	s *SyncHandler,
	userID string,
	noteCloudIDHex string,
) ([]string, error) {
	prefix := userID + "/attachments/"
	attachmentIDs := []string{}

	paginator := s3.NewListObjectsV2Paginator(s.s3Client, &s3.ListObjectsV2Input{
		Bucket: aws.String(s.s3Bucket),
		Prefix: aws.String(prefix),
	})

	for paginator.HasMorePages() {
		page, err := paginator.NextPage(ctx)
		if err != nil {
			return nil, fmt.Errorf(
				"list attachments for note %s: %w",
				noteCloudIDHex,
				err,
			)
		}

		for _, obj := range page.Contents {
			if err := s.acquireWorker(ctx); err != nil {
				return nil, err
			}

			headOut, err := s.s3Client.HeadObject(ctx, &s3.HeadObjectInput{
				Bucket: aws.String(s.s3Bucket),
				Key:    obj.Key,
			})

			s.releaseWorker()

			if err != nil {
				log.Errorw(
					"list attachments for note: head object failed, skipping",
					"key", *obj.Key,
					"note_cloud_id", noteCloudIDHex,
					"error", err,
				)
				continue
			}

			if headOut.Metadata["note_cloud_id"] != noteCloudIDHex {
				continue
			}

			attachmentIDs = append(
				attachmentIDs,
				strings.TrimPrefix(*obj.Key, prefix),
			)
		}
	}

	return attachmentIDs, nil
}

func cleanupDeletedNoteAttachments(
	note models.DownloadNote,
	ctx context.Context,
	userID string,
	s *SyncHandler,
) error {
	if len(note.DeletedAttachments) == 0 {
		return nil
	}

	g, ctx := errgroup.WithContext(ctx)

	for _, attachmentID := range note.DeletedAttachments {
		attachmentID := attachmentID

		g.Go(func() error {
			cloudKey := createObjectKey(
				attachmentID,
				userID,
			)

			if err := deleteAttachmentObject(
				ctx,
				s,
				cloudKey,
			); err != nil {
				return fmt.Errorf(
					"delete attachment %s: %w",
					attachmentID,
					err,
				)
			}

			return nil
		})
	}

	if err := g.Wait(); err != nil {
		return err
	}

	return nil
}

func deleteAttachmentObject(
	ctx context.Context,
	s *SyncHandler,
	cloudKey string,
) error {
	if err := s.acquireWorker(ctx); err != nil {
		return err
	}

	defer s.releaseWorker()

	_, err := s.s3Client.DeleteObject(
		ctx,
		&s3.DeleteObjectInput{
			Bucket: aws.String(s.s3Bucket),
			Key:    aws.String(cloudKey),
		},
	)

	if err != nil {
		return fmt.Errorf(
			"delete S3 object %q: %w",
			cloudKey,
			err,
		)
	}

	return nil
}

func checkAttachment(
	attachment models.AttachmentSyncCheck,
	userID string,
	ctx context.Context,
	s *SyncHandler,
	cloudID string,
) (bool, []models.UploadAttachment, []models.DownloadAttachment, []string, []string, error) {
	attachmentsToUpload := []models.UploadAttachment{}
	attachmentsToDownload := []models.DownloadAttachment{}
	syncedAttachments := []string{}
	attachmentsToHardDelete := []string{}
	quotaExceeded := false
	if attachment.HardDeleted {
		key := createObjectKey(
			attachment.AttachmentID.String(),
			userID,
		)

		if err := deleteBrokenAttachment(
			key,
			s.s3Client,
			ctx,
			s.s3Bucket,
			s,
		); err != nil {
			return quotaExceeded, nil, nil, nil, nil, fmt.Errorf(
				"delete hard-deleted attachment %q: %w",
				key,
				err,
			)
		}

		cloudObjectID, err := bson.ObjectIDFromHex(cloudID)
		if err != nil {
			return quotaExceeded, nil, nil, nil, nil, fmt.Errorf(
				"invalid cloud id %q: %w",
				cloudID,
				err,
			)
		}

		filter := bson.M{
			"_id":      cloudObjectID,
			"owner_id": userID,
		}

		update := bson.M{
			"$addToSet": bson.M{
				"deleted_attachments": attachment.AttachmentID.String(),
			},
		}

		if err := s.acquireWorker(ctx); err != nil {
			return quotaExceeded, nil, nil, nil, nil, err
		}

		result := s.Coll.FindOneAndUpdate(
			ctx,
			filter,
			update,
			options.FindOneAndUpdate().SetReturnDocument(options.After),
		)

		s.releaseWorker()

		updated := new(models.DownloadNote)

		if err := result.Decode(updated); err != nil {
			if errors.Is(err, mongo.ErrNoDocuments) {
				return quotaExceeded, nil, nil, nil, nil, fmt.Errorf(
					"note %s not found for attachment tombstone update: %w",
					cloudID,
					err,
				)
			}

			return quotaExceeded, nil, nil, nil, nil, fmt.Errorf(
				"update deleted_attachments for note %s: %w",
				cloudID,
				err,
			)
		}

		attachmentsToHardDelete = append(
			attachmentsToHardDelete,
			attachment.AttachmentID.String(),
		)

		return quotaExceeded, nil, nil, nil, attachmentsToHardDelete, nil
	}

	cloudKey := createObjectKey(
		attachment.AttachmentID.String(),
		userID,
	)

	if err := s.acquireWorker(ctx); err != nil {
		return quotaExceeded, nil, nil, nil, nil, err
	}

	foundAttachment, err := s.s3Client.HeadObject(
		ctx,
		&s3.HeadObjectInput{
			Bucket: aws.String(s.s3Bucket),
			Key:    aws.String(cloudKey),
		},
	)

	s.releaseWorker()

	if err != nil {
		var apiErr smithy.APIError

		if errors.As(err, &apiErr) && apiErr.ErrorCode() == "NotFound" {
			cloudObjectID, err := bson.ObjectIDFromHex(cloudID)
			if err != nil {
				return quotaExceeded, nil, nil, nil, nil, fmt.Errorf(
					"invalid cloud id %q: %w",
					cloudID,
					err,
				)
			}

			if err := s.acquireWorker(ctx); err != nil {
				return quotaExceeded, nil, nil, nil, nil, err
			}

			existsCheck := new(bson.M)

			findErr := s.Coll.FindOne(
				ctx,
				bson.M{
					"_id":                 cloudObjectID,
					"owner_id":            userID,
					"deleted_attachments": attachment.AttachmentID.String(),
				},
				options.FindOne().SetProjection(
					bson.M{"_id": 1},
				),
			).Decode(existsCheck)

			s.releaseWorker()

			if findErr != nil && !errors.Is(findErr, mongo.ErrNoDocuments) {
				return quotaExceeded, nil, nil, nil, nil, fmt.Errorf(
					"check deleted_attachments for attachment %s: %w",
					attachment.AttachmentID.String(),
					findErr,
				)
			}

			if findErr == nil {
				attachmentsToHardDelete = append(
					attachmentsToHardDelete,
					attachment.AttachmentID.String(),
				)

				return quotaExceeded, nil, nil, nil, attachmentsToHardDelete, nil
			}

			metadata, err := createAttachmentMetadata(
				attachment,
				cloudID,
			)

			if err != nil {
				return quotaExceeded, nil, nil, nil, nil, fmt.Errorf(
					"create metadata for attachment %s: %w",
					attachment.AttachmentID.String(),
					err,
				)
			}
			err = checkIfUploadPossible(s, attachment.SizeBytes, ctx, userID)

			if err != nil {
				if errors.Is(err, ErrQuotaExceeded) {
					return true, nil, nil, nil, nil, nil
				}

				return quotaExceeded, nil, nil, nil, nil, fmt.Errorf(
					"upload not possible for attachment %s: %w",
					attachment.AttachmentID.String(),
					err,
				)
			}

			url, err := createUploadURL(
				ctx,
				s.presigner,
				s.s3Bucket,
				cloudKey,
				metadata,
				s, userID, attachment.AttachmentID.String(), attachment.SizeBytes,
			)

			if err != nil {
				return quotaExceeded, nil, nil, nil, nil, fmt.Errorf(
					"create upload URL for attachment %s: %w",
					attachment.AttachmentID.String(),
					err,
				)
			}

			attachmentsToUpload = append(
				attachmentsToUpload,
				models.UploadAttachment{
					AttachmentId: attachment.AttachmentID.String(),
					UploadUrl:    url,
				},
			)

			return quotaExceeded, attachmentsToUpload, nil, nil, nil, nil
		}

		return quotaExceeded, nil, nil, nil, nil, fmt.Errorf(
			"head object %q: %w",
			cloudKey,
			err,
		)
	}

	metadata, err := parseAttachmentMetadata(
		foundAttachment.Metadata,
		cloudID,
	)

	if err != nil {
		log.Debugw(
			"Malformed attachment, deleting object",
			"key",
			cloudKey,
			"error",
			err,
		)

		if deleteErr := deleteBrokenAttachment(
			cloudKey,
			s.s3Client,
			ctx,
			s.s3Bucket,
			s,
		); deleteErr != nil {
			return quotaExceeded, nil, nil, nil, nil, fmt.Errorf(
				"delete malformed attachment %q: %w",
				cloudKey,
				deleteErr,
			)
		}

		return quotaExceeded, nil, nil, nil, nil, fmt.Errorf(
			"parse attachment metadata %q: %w",
			cloudKey,
			err,
		)
	}

	if metadata.ChecksumEncrypted == attachment.ChecksumEncrypted {
		syncedAttachments = append(
			syncedAttachments,
			attachment.AttachmentID.String(),
		)

		return quotaExceeded, nil, nil, syncedAttachments, nil, nil
	}

	downloadURL, err := s.presigner.PresignGetObject(
		ctx,
		&s3.GetObjectInput{
			Bucket: aws.String(s.s3Bucket),
			Key:    aws.String(cloudKey),
		},
		s3.WithPresignExpires(10*time.Minute),
	)

	if err != nil {
		return quotaExceeded, nil, nil, nil, nil, fmt.Errorf(
			"create download URL for attachment %s: %w",
			attachment.AttachmentID.String(),
			err,
		)
	}

	attachmentStruct := models.DownloadAttachment{
		AttachmentID:      attachment.AttachmentID,
		FileName:          metadata.FileName,
		MimeType:          metadata.MimeType,
		SizeBytes:         metadata.SizeBytes,
		CloudKey:          cloudKey,
		CloudNoteId:       metadata.NoteCloudID,
		ChecksumEncrypted: metadata.ChecksumEncrypted,
		IsEncrypted:       metadata.IsEncrypted,
		CryptoMeta:        metadata.CryptoMeta,
		CreatedAt:         metadata.CreatedAt,
		UpdatedAt:         metadata.UpdatedAt,
		DownloadUrl:       downloadURL.URL,
	}

	attachmentsToDownload = append(
		attachmentsToDownload,
		attachmentStruct,
	)

	return quotaExceeded, nil, attachmentsToDownload, nil, nil, nil
}

type parsedAttachmentMetadata struct {
	ChecksumEncrypted string
	SizeBytes         int64
	IsEncrypted       bool
	FileName          string
	NoteCloudID       string
	MimeType          string
	CryptoMeta        *models.AttachmentCryptoMeta
	CreatedAt         int64
	UpdatedAt         int64
}

func parseAttachmentMetadata(
	metadata map[string]string,
	expectedNoteCloudID string,
) (parsedAttachmentMetadata, error) {
	checksum, ok := metadata["checksum"]
	if !ok || checksum == "" {
		return parsedAttachmentMetadata{}, errors.New("missing checksum")
	}

	sizeBytesString, ok := metadata["size_bytes"]
	if !ok || sizeBytesString == "" {
		return parsedAttachmentMetadata{}, errors.New("missing size_bytes")
	}

	sizeBytes, err := strconv.ParseInt(sizeBytesString, 10, 64)
	if err != nil {
		return parsedAttachmentMetadata{}, errors.New("invalid size_bytes")
	}

	if sizeBytes < 0 {
		return parsedAttachmentMetadata{}, errors.New("negative size_bytes")
	}

	isEncryptedString, ok := metadata["is_encrypted"]
	if !ok || isEncryptedString == "" {
		return parsedAttachmentMetadata{}, errors.New("missing is_encrypted")
	}

	isEncrypted, err := strconv.ParseBool(isEncryptedString)
	if err != nil {
		return parsedAttachmentMetadata{}, errors.New("invalid is_encrypted")
	}

	fileName, ok := metadata["file_name"]
	if !ok || fileName == "" {
		return parsedAttachmentMetadata{}, errors.New("missing file_name")
	}

	mimeType, ok := metadata["mime_type"]
	if !ok || mimeType == "" {
		return parsedAttachmentMetadata{}, errors.New("missing mime_type")
	}

	noteCloudID, ok := metadata["note_cloud_id"]
	if !ok || noteCloudID == "" {
		return parsedAttachmentMetadata{}, errors.New("missing note_cloud_id")
	}

	if noteCloudID != expectedNoteCloudID {
		return parsedAttachmentMetadata{}, errors.New(
			"attachment belongs to another note",
		)
	}

	createdAtString, ok := metadata["created_at"]
	if !ok || createdAtString == "" {
		return parsedAttachmentMetadata{}, errors.New("missing created_at")
	}

	createdAt, err := strconv.ParseInt(createdAtString, 10, 64)
	if err != nil {
		return parsedAttachmentMetadata{}, errors.New("invalid created_at")
	}

	updatedAtString, ok := metadata["updated_at"]
	if !ok || updatedAtString == "" {
		return parsedAttachmentMetadata{}, errors.New("missing updated_at")
	}

	updatedAt, err := strconv.ParseInt(updatedAtString, 10, 64)
	if err != nil {
		return parsedAttachmentMetadata{}, errors.New("invalid updated_at")
	}

	parsed := parsedAttachmentMetadata{
		ChecksumEncrypted: checksum,
		SizeBytes:         sizeBytes,
		IsEncrypted:       isEncrypted,
		NoteCloudID:       noteCloudID,
		FileName:          fileName,
		MimeType:          mimeType,
		CreatedAt:         createdAt,
		UpdatedAt:         updatedAt,
	}

	cryptoMetaJSON := metadata["crypto_meta"]

	if isEncrypted {
		if cryptoMetaJSON == "" {
			return parsedAttachmentMetadata{}, errors.New(
				"missing crypto_meta for encrypted attachment",
			)
		}

		cryptoMeta := new(models.AttachmentCryptoMeta)

		if err := json.Unmarshal(
			[]byte(cryptoMetaJSON),
			cryptoMeta,
		); err != nil {
			return parsedAttachmentMetadata{}, errors.New(
				"invalid crypto_meta",
			)
		}

		parsed.CryptoMeta = cryptoMeta
	} else if cryptoMetaJSON != "" {
		cryptoMeta := new(models.AttachmentCryptoMeta)

		if err := json.Unmarshal(
			[]byte(cryptoMetaJSON),
			cryptoMeta,
		); err != nil {
			return parsedAttachmentMetadata{}, errors.New(
				"invalid crypto_meta",
			)
		}

		parsed.CryptoMeta = cryptoMeta
	}

	return parsed, nil
}

func deleteBrokenAttachment(
	s3Key string,
	s3Client *s3.Client,
	ctx context.Context,
	s3Bucket string,
	s *SyncHandler,
) error {
	if err := s.acquireWorker(ctx); err != nil {
		return err
	}

	defer s.releaseWorker()

	_, err := s3Client.DeleteObject(
		ctx,
		&s3.DeleteObjectInput{
			Bucket: aws.String(s3Bucket),
			Key:    aws.String(s3Key),
		},
	)

	return err
}

func createUploadURL(
	ctx context.Context,
	presigner *s3.PresignClient,
	bucket string,
	key string,
	metadata map[string]string,
	s *SyncHandler,
	userID string,
	attachmentID string,
	sizeBytes int64,
) (string, error) {

	request, err := presigner.PresignPutObject(
		ctx,
		&s3.PutObjectInput{
			Bucket:   aws.String(bucket),
			Key:      aws.String(key),
			Metadata: metadata,
		},
		s3.WithPresignExpires(10*time.Minute),
	)

	if err != nil {
		return "", err
	}
	rstruct := models.QuotaReservation{
		UserID:       userID,
		AttachmentID: attachmentID,
		SizeBytes:    sizeBytes,
		Status:       string(ReservationPending),
		ExpiresAt:    time.Now().Add(10 * time.Minute),
		CreatedAt:    time.Now(),
	}
	s.acquireWorker(ctx)
	defer s.releaseWorker()
	reservations := s.DB.Collection("reservations")
	_, err = reservations.InsertOne(ctx, rstruct)

	if err != nil {
		return "", err
	}

	return request.URL, nil

}

func createAttachmentMetadata(
	attachment models.AttachmentSyncCheck,
	noteCloudID string,
) (map[string]string, error) {
	cryptoMeta := ""

	if attachment.IsEncrypted {
		cryptoMetaBytes, err := json.Marshal(
			attachment.CryptoMetadata,
		)
		if err != nil {
			return nil, err
		}

		cryptoMeta = string(cryptoMetaBytes)
	}

	if noteCloudID == "" {
		return nil, errors.New("note cloud ID is empty")
	}

	metadata := map[string]string{
		"checksum":      attachment.ChecksumEncrypted,
		"size_bytes":    strconv.FormatInt(attachment.SizeBytes, 10),
		"is_encrypted":  strconv.FormatBool(attachment.IsEncrypted),
		"file_name":     attachment.FileName,
		"mime_type":     attachment.MimeType,
		"crypto_meta":   cryptoMeta,
		"note_cloud_id": noteCloudID,
		"created_at":    strconv.FormatInt(attachment.CreatedAt, 10),
		"updated_at":    strconv.FormatInt(attachment.UpdatedAt, 10),
	}

	return metadata, nil
}

func createObjectKey(id string, userID string) string {
	return fmt.Sprintf("%s/attachments/%s", userID, id)
}

func checkNotesNotExistingOnClient(
	s *SyncHandler,
	ctx context.Context,
	userID string,
	allNotes []models.CheckNoteSyncStatus,
) ([]models.DownloadNote, []models.DownloadAttachment, error) {
	notesToDownload := []models.DownloadNote{}

	if err := s.acquireWorker(ctx); err != nil {
		return nil, nil, err
	}

	defer s.releaseWorker()

	cursor, err := s.Coll.Find(
		ctx,
		bson.M{"owner_id": userID},
	)
	if err != nil {
		return nil, nil, err
	}

	var cloudNotes []models.DownloadNote
	if err := cursor.All(ctx, &cloudNotes); err != nil {
		return nil, nil, err
	}

	clientCloudIDs := make(map[string]struct{}, len(allNotes))
	for _, note := range allNotes {
		if note.CloudID != nil {
			clientCloudIDs[note.CloudID.String()] = struct{}{}
		}
	}

	missingNoteIDs := make(map[string]bool)
	for _, cloudNote := range cloudNotes {
		if cloudNote.HardDeleted {
			continue
		}
		if _, exists := clientCloudIDs[cloudNote.CloudID.String()]; !exists {
			notesToDownload = append(notesToDownload, cloudNote)
			missingNoteIDs[cloudNote.CloudID.Hex()] = true
		}
	}

	var attachmentsToDownload []models.DownloadAttachment
	if len(missingNoteIDs) > 0 {
		prefix := userID + "/attachments/"
		if s.s3Bucket == "" {
			log.Error("Bucket is empty")
		}

		log.Infow("Listing S3 attachments",
			"bucket", s.s3Bucket,
			"prefix", prefix,
		)

		paginator := s3.NewListObjectsV2Paginator(s.s3Client, &s3.ListObjectsV2Input{
			Bucket: aws.String(s.s3Bucket),
			Prefix: aws.String(prefix),
		})

		for paginator.HasMorePages() {
			page, err := paginator.NextPage(ctx)
			if err != nil {
				log.Errorw("failed to list attachments during full sync", "error", err)
				break
			}

			for _, obj := range page.Contents {
				if err := s.acquireWorker(ctx); err != nil {
					continue
				}

				headOut, err := s.s3Client.HeadObject(ctx, &s3.HeadObjectInput{
					Bucket: aws.String(s.s3Bucket),
					Key:    obj.Key,
				})

				s.releaseWorker()

				if err != nil {
					continue
				}

				noteCloudID, ok := headOut.Metadata["note_cloud_id"]
				if !ok || !missingNoteIDs[noteCloudID] {
					continue
				}

				metadata, err := parseAttachmentMetadata(headOut.Metadata, noteCloudID)
				if err != nil {
					log.Debugw("malformed attachment during full sync scan", "key", *obj.Key, "error", err)

					if delErr := deleteBrokenAttachment(*obj.Key, s.s3Client, ctx, s.s3Bucket, s); delErr != nil {
						log.Errorw("failed to delete malformed attachment", "key", *obj.Key, "error", delErr)
					}
					continue
				}

				downloadURL, err := s.presigner.PresignGetObject(
					ctx,
					&s3.GetObjectInput{
						Bucket: aws.String(s.s3Bucket),
						Key:    obj.Key,
					},
					s3.WithPresignExpires(10*time.Minute),
				)
				if err != nil {
					log.Errorw("failed to presign attachment download", "key", *obj.Key, "error", err)
					continue
				}

				attachmentIDStr := strings.TrimPrefix(*obj.Key, prefix)
				attachmentUUID, err := uuid.Parse(attachmentIDStr)
				if err != nil {
					log.Debugw("invalid attachment key format", "key", *obj.Key, "error", err)
					continue
				}

				attachmentsToDownload = append(attachmentsToDownload, models.DownloadAttachment{
					AttachmentID:      attachmentUUID,
					FileName:          metadata.FileName,
					MimeType:          metadata.MimeType,
					SizeBytes:         metadata.SizeBytes,
					CloudKey:          *obj.Key,
					CloudNoteId:       noteCloudID,
					ChecksumEncrypted: metadata.ChecksumEncrypted,
					IsEncrypted:       metadata.IsEncrypted,
					CryptoMeta:        metadata.CryptoMeta,
					CreatedAt:         metadata.CreatedAt,
					UpdatedAt:         metadata.UpdatedAt,
					DownloadUrl:       downloadURL.URL,
				})
			}
		}
	}

	return notesToDownload, attachmentsToDownload, nil
}

type NoteDocument struct {
	OwnerID      string                 `bson:"owner_id"`
	CloudVersion int64                  `bson:"cloud_version"`
	Title        string                 `bson:"title"`
	Summary      string                 `bson:"summary"`
	Content      string                 `bson:"content"`
	CreatedAt    int64                  `bson:"created_at"`
	UpdatedAt    int64                  `bson:"updated_at"`
	IsDeleted    bool                   `bson:"is_deleted"`
	DeletedAt    *int64                 `bson:"deleted_at,omitempty"`
	HardDeleted  bool                   `bson:"hard_deleted"`
	IsEncrypted  bool                   `bson:"is_encrypted"`
	CryptoMeta   *models.NoteCryptoMeta `bson:"crypto_meta,omitempty"`
}

func (s *SyncHandler) UploadNote(c fiber.Ctx) error {
	req := new(models.NoteUploadRequest)

	if err := c.Bind().Body(req); err != nil {
		log.Errorw("upload note: failed to parse body", "error", err)
		return middleware.BadRequest("Couldn't read body data")
	}

	if err := s.Validator.Struct(req); err != nil {
		log.Errorw("upload note: validation failed", "error", err, "title", req.Title)
		return middleware.BadRequest("Wrong upload note struct was sent")
	}

	ownerID := c.Locals("userID").(string)

	log.Debugw(
		"upload note request received",
		"owner_id", ownerID,
		"title", req.Title,
		"is_encrypted", req.IsEncrypted,
		"is_deleted", req.IsDeleted,
		"created_at", req.CreatedAt,
		"updated_at", req.UpdatedAt,
	)

	var crypto *models.NoteCryptoMeta
	if req.CryptoMeta != nil {
		if err := json.Unmarshal([]byte(*req.CryptoMeta), &crypto); err != nil {
			log.Errorw(
				"upload note: invalid crypto_meta",
				"owner_id", ownerID,
				"title", req.Title,
				"error", err,
			)
			return middleware.BadRequest("Invalid crypto_meta")
		}
	}

	doc := NoteDocument{
		OwnerID:      ownerID,
		CloudVersion: 1,
		Title:        req.Title,
		Summary:      req.Summary,
		Content:      req.Content,
		CreatedAt:    req.CreatedAt,
		UpdatedAt:    req.UpdatedAt,
		IsDeleted:    req.IsDeleted,
		DeletedAt:    req.DeletedAt,
		IsEncrypted:  req.IsEncrypted,
		HardDeleted:  req.HardDeleted,
		CryptoMeta:   crypto,
	}

	ctx, cancel := context.WithTimeout(context.Background(), 50*time.Second)
	defer cancel()

	res, err := s.Coll.InsertOne(ctx, doc)
	if err != nil {
		log.Errorw(
			"upload note: mongo insert failed",
			"owner_id", ownerID,
			"title", req.Title,
			"error", err,
		)
		return middleware.Internal("Failed to add note to mongo db")
	}

	insertedID, ok := res.InsertedID.(bson.ObjectID)
	if !ok {
		log.Errorw(
			"upload note: inserted id was not an ObjectID",
			"owner_id", ownerID,
			"title", req.Title,
			"inserted_id", res.InsertedID,
		)
		return middleware.Internal("Failed to read inserted note id")
	}

	log.Debugw(
		"upload note: inserted successfully",
		"owner_id", ownerID,
		"mongo_id", insertedID.Hex(),
		"title", req.Title,
	)

	return c.JSON(fiber.Map{
		"mongo_id":      insertedID.Hex(),
		"cloud_version": 1,
	})
}

func (s *SyncHandler) UpdateNote(c fiber.Ctx) error {
	mongoID := c.Params("mongo_id")

	objectID, err := bson.ObjectIDFromHex(mongoID)
	if err != nil {
		log.Errorw(
			"update note: invalid mongo_id in path",
			"mongo_id", mongoID,
			"error", err,
		)

		return middleware.BadRequest("Invalid mongo_id in path")
	}

	req := new(models.NoteUploadRequest)

	if err := c.Bind().Body(req); err != nil {
		log.Errorw(
			"update note: failed to parse body",
			"mongo_id", mongoID,
			"error", err,
		)

		return middleware.BadRequest("Couldn't read body data")
	}

	if err := s.Validator.Struct(req); err != nil {
		log.Errorw(
			"update note: validation failed",
			"mongo_id", mongoID,
			"error", err,
		)

		return middleware.BadRequest("Wrong update note struct was sent")
	}

	ownerID, ok := c.Locals("userID").(string)
	if !ok || ownerID == "" {
		log.Errorw(
			"update note: missing authenticated user",
			"mongo_id", mongoID,
		)

		return middleware.Unauthorized("Missing authenticated user")
	}

	log.Debugw(
		"update note request received",
		"owner_id", ownerID,
		"mongo_id", mongoID,
		"title", req.Title,
	)

	var crypto *models.NoteCryptoMeta

	if req.CryptoMeta != nil {
		if err := json.Unmarshal([]byte(*req.CryptoMeta), &crypto); err != nil {
			log.Errorw(
				"update note: invalid crypto_meta",
				"owner_id", ownerID,
				"mongo_id", mongoID,
				"error", err,
			)

			return middleware.BadRequest("Invalid crypto_meta")
		}
	}

	ctx, cancel := context.WithTimeout(
		context.Background(),
		50*time.Second,
	)
	defer cancel()

	filter := bson.M{
		"_id":      objectID,
		"owner_id": ownerID,
	}

	update := bson.M{
		"$set": bson.M{
			"title":        req.Title,
			"summary":      req.Summary,
			"content":      req.Content,
			"created_at":   req.CreatedAt,
			"updated_at":   req.UpdatedAt,
			"is_deleted":   req.IsDeleted,
			"deleted_at":   req.DeletedAt,
			"is_encrypted": req.IsEncrypted,
			"crypto_meta":  crypto,
		},
		"$inc": bson.M{
			"cloud_version": 1,
		},
	}

	var updatedNote NoteDocument

	err = s.Coll.FindOneAndUpdate(
		ctx,
		filter,
		update,
		options.FindOneAndUpdate().
			SetReturnDocument(options.After),
	).Decode(&updatedNote)

	if err != nil {
		if errors.Is(err, mongo.ErrNoDocuments) {
			log.Debugw(
				"update note: no matching note found for owner",
				"owner_id", ownerID,
				"mongo_id", mongoID,
			)

			return middleware.BadRequest("Note not found")
		}

		log.Errorw(
			"update note: mongo update failed",
			"owner_id", ownerID,
			"mongo_id", mongoID,
			"error", err,
		)

		return middleware.Internal("Failed to update note in mongo db")
	}

	log.Debugw(
		"update note: updated successfully",
		"owner_id", ownerID,
		"mongo_id", mongoID,
		"cloud_version", updatedNote.CloudVersion,
	)

	return c.JSON(fiber.Map{
		"cloud_version": updatedNote.CloudVersion,
	})
}

func checkIfUploadPossible(s *SyncHandler, size int64, ctx context.Context, userID string) error {
	if size <= 0 {
		return middleware.BadRequest("ATTACHMENT SIZE CAN NOT BE LESS THAN 0")
	}

	userObjectID, err := bson.ObjectIDFromHex(userID)
	if err != nil {
		return middleware.BadRequest("invalid user id")
	}

	s.acquireWorker(ctx)
	defer s.releaseWorker()

	usersDB := s.DB.Collection("users_data")

	var result struct {
		QuotaUsed int64 `bson:"quota_used_bytes"`
	}

	err = usersDB.FindOne(
		ctx,
		bson.M{
			"_id": userObjectID,
		},
		options.FindOne().SetProjection(bson.M{
			"quota_used_bytes": 1,
			"_id":              0,
		}),
	).Decode(&result)

	if err != nil {
		log.Errorw("failed to check quota used", "error", err)
		return err
	}

	reservations := s.DB.Collection("reservations")

	pipeline := mongo.Pipeline{
		{
			{Key: "$match", Value: bson.M{
				"user_id": userID,
				"status":  string(ReservationPending),
				"expires_at": bson.M{
					"$gt": time.Now(),
				},
			}},
		},
		{
			{Key: "$group", Value: bson.M{
				"_id":        nil,
				"total_size": bson.M{"$sum": "$size_bytes"},
			}},
		},
	}

	var reserved struct {
		TotalSize int64 `bson:"total_size"`
	}

	cursor, err := reservations.Aggregate(ctx, pipeline)
	if err != nil {
		return err
	}
	defer cursor.Close(ctx)

	if cursor.Next(ctx) {
		if err := cursor.Decode(&reserved); err != nil {
			return err
		}
	}

	if err := cursor.Err(); err != nil {
		return err
	}

	const quotaLimit int64 = 100 * 1024 * 1024

	if result.QuotaUsed+reserved.TotalSize+size > quotaLimit {
		return ErrQuotaExceeded
	}

	return nil
}
func (s *SyncHandler) manageReservationAndQuota(c fiber.Ctx) error {
	log.Debug("IN CONSUMING")
	attachmentID := c.Params("attachment_id")
	userID := c.Locals("userID").(string)

	key := createObjectKey(attachmentID, userID)

	// 1. Verify object exists in S3
	s.acquireWorker(c)

	headOut, err := s.s3Client.HeadObject(c, &s3.HeadObjectInput{
		Bucket: &s.s3Bucket,
		Key:    &key,
	})

	s.releaseWorker()

	if err != nil {
		return middleware.NotFound("uploaded object does not exist")
	}

	if headOut.ContentLength == nil {
		return middleware.Internal("missing object size")
	}

	actualSize := *headOut.ContentLength

	reservations := s.DB.Collection("reservations")
	users := s.DB.Collection("users_data")

	var reservation models.QuotaReservation

	err = reservations.FindOne(
		c,
		bson.M{
			"attachment_id": attachmentID,
			"user_id":       userID,
			"status":        string(ReservationPending),
			"expires_at": bson.M{
				"$gt": time.Now(),
			},
		},
	).Decode(&reservation)

	if err != nil {
		if errors.Is(err, mongo.ErrNoDocuments) {
			// Already consumed, expired or does not exist.
			return c.SendStatus(fiber.StatusOK)
		}

		return middleware.Internal("failed to find reservation")
	}

	if actualSize != reservation.SizeBytes {
		return middleware.BadRequest("uploaded object size does not match reservation")
	}

	// 4. Mongo transaction
	s.acquireWorker(c)
	defer s.releaseWorker()

	session, err := s.DB.Client().StartSession()
	if err != nil {
		return middleware.Internal("failed to start Mongo session")
	}
	defer session.EndSession(c)
	userObjectID, err := bson.ObjectIDFromHex(userID)
	if err != nil {
		return middleware.BadRequest("invalid user id")
	}
	_, err = session.WithTransaction(
		c,
		func(sc context.Context) (any, error) {
			// 1. Atomically consume the reservation.
			result, err := reservations.UpdateOne(
				sc,
				bson.M{
					"_id":    reservation.ID,
					"status": string(ReservationPending),
				},
				bson.M{
					"$set": bson.M{
						"status":      string(ReservationConsumed),
						"consumed_at": time.Now(),
					},
				},
			)
			if err != nil {
				return nil, err
			}
			if result.ModifiedCount != 1 {
				// Worker or another request already consumed it.
				return nil, nil
			}

			// 2. Increment user's tracked usage.
			_, err = users.UpdateOne(
				sc,
				bson.M{"_id": userObjectID},
				bson.M{
					"$inc": bson.M{
						"quota_used_bytes": actualSize,
					},
				},
			)
			if err != nil {
				return nil, err
			}

			return nil, nil
		},
	)
	if err != nil {
		return middleware.Internal("failed to finalize upload")
	}

	return c.SendStatus(fiber.StatusOK)
}

// StartReservationCleanupWorker starts a background goroutine that periodically
// reconciles expired pending reservations. It should be called once at application
// startup. The worker runs immediately and then every 5 minutes until the
// provided context is cancelled.
func (s *SyncHandler) StartReservationCleanupWorker(ctx context.Context) {
	go func() {
		ticker := time.NewTicker(5 * time.Minute)
		defer ticker.Stop()

		// Run the first reconciliation immediately on startup.
		s.runCleanupIteration(ctx)

		for {
			select {
			case <-ctx.Done():
				log.Infow("reservation cleanup worker stopped")
				return
			case <-ticker.C:
				s.runCleanupIteration(ctx)
			}
		}
	}()
}

// runCleanupIteration creates a bounded timeout context for a single sweep
// so that a slow S3/Mongo call cannot hang the worker forever.
func (s *SyncHandler) runCleanupIteration(parentCtx context.Context) {
	ctx, cancel := context.WithTimeout(parentCtx, 3*time.Minute)
	defer cancel()

	if err := s.cleanupExpiredReservations(ctx); err != nil {
		log.Errorw("reservation cleanup iteration failed", "error", err)
	}
}

// cleanupExpiredReservations finds all pending reservations whose expires_at
// has passed and processes them in batches.
func (s *SyncHandler) cleanupExpiredReservations(ctx context.Context) error {
	reservations := s.DB.Collection("reservations")
	// Select only pending reservations that have already expired.
	filter := bson.M{
		"status":     string(ReservationPending),
		"expires_at": bson.M{"$lte": time.Now()},
	}

	cursor, err := reservations.Find(ctx, filter)
	if err != nil {
		return fmt.Errorf("find expired reservations: %w", err)
	}
	defer cursor.Close(ctx)

	// Collect reservations in small batches so we can process them with
	// bounded concurrency and avoid loading huge result sets into memory.
	const batchSize = 50
	var batch []models.QuotaReservation

	for cursor.Next(ctx) {
		var r models.QuotaReservation
		if err := cursor.Decode(&r); err != nil {
			log.Errorw("failed to decode reservation", "error", err)
			continue
		}
		batch = append(batch, r)

		if len(batch) >= batchSize {
			if err := s.processReservationBatch(ctx, batch); err != nil {
				log.Errorw("reservation batch processing failed", "error", err)
			}
			batch = batch[:0]
		}
	}

	// Process any trailing items.
	if len(batch) > 0 {
		if err := s.processReservationBatch(ctx, batch); err != nil {
			log.Errorw("final reservation batch processing failed", "error", err)
		}
	}

	return cursor.Err()
}

// processReservationBatch reconciles a slice of expired reservations with
// limited parallelism (max 10 concurrent) so we do not hammer S3 or MongoDB.
func (s *SyncHandler) processReservationBatch(ctx context.Context, batch []models.QuotaReservation) error {
	g, ctx := errgroup.WithContext(ctx)
	g.SetLimit(10)

	for _, r := range batch {
		r := r
		g.Go(func() error {
			if err := s.reconcileSingleReservation(ctx, r); err != nil {
				log.Errorw("single reservation reconciliation failed",
					"attachment_id", r.AttachmentID,
					"user_id", r.UserID,
					"error", err)
			}
			// Never let one bad reservation abort the whole batch.
			return nil
		})
	}

	return g.Wait()
}

// reconcileSingleReservation decides what to do with one expired pending
// reservation by checking S3 first, then acting on MongoDB.
//
// Rules:
//  1. Object missing on S3          → delete the orphaned reservation.
//  2. Object exists, size mismatch  → delete both S3 object and reservation (garbage).
//  3. Object exists, size matches   → run an idempotent transaction to
//     mark the reservation consumed and increment the user's quota_used_bytes.
func (s *SyncHandler) reconcileSingleReservation(ctx context.Context, res models.QuotaReservation) error {
	key := createObjectKey(res.AttachmentID, res.UserID)

	// 1. Check S3 first. If the object is not there, the reservation is orphaned.
	if err := s.acquireWorker(ctx); err != nil {
		return err
	}

	headOut, err := s.s3Client.HeadObject(ctx, &s3.HeadObjectInput{
		Bucket: aws.String(s.s3Bucket),
		Key:    aws.String(key),
	})

	s.releaseWorker()

	if err != nil {
		var apiErr smithy.APIError
		if errors.As(err, &apiErr) && apiErr.ErrorCode() == "NotFound" {
			_, delErr := s.DB.Collection("reservations").DeleteOne(ctx, bson.M{"_id": res.ID})
			if delErr != nil {
				return fmt.Errorf("delete orphaned reservation %s: %w", res.AttachmentID, delErr)
			}
			log.Debugw("cleaned up orphaned reservation (S3 object missing)",
				"attachment_id", res.AttachmentID)
			return nil
		}
		return fmt.Errorf("head object %s: %w", key, err)
	}

	if headOut.ContentLength == nil {
		// Corrupted / incomplete metadata. Treat as garbage.
		s.deleteS3ObjectSilent(ctx, key)
		s.DB.Collection("reservations").DeleteOne(ctx, bson.M{"_id": res.ID})
		return fmt.Errorf("head object %s missing content-length", key)
	}

	actualSize := *headOut.ContentLength

	// 2. Size mismatch means the client uploaded something unexpected.
	// Clean up both sides to prevent quota and storage leaks.
	if actualSize != res.SizeBytes {
		log.Warnw("reservation size mismatch, treating as garbage",
			"attachment_id", res.AttachmentID,
			"expected_size", res.SizeBytes,
			"actual_size", actualSize)

		s.deleteS3ObjectSilent(ctx, key)
		s.DB.Collection("reservations").DeleteOne(ctx, bson.M{"_id": res.ID})
		return nil
	}

	// 3. Object exists and size matches → finalize exactly like manageReservationAndQuota.
	return s.finalizeReservationTransaction(ctx, res, actualSize, res.UserID)
}

// finalizeReservationTransaction atomically consumes a pending reservation and
// increments the user's quota_used_bytes. It is fully idempotent: if another
// request (e.g. the client's manageReservationAndQuota call) already flipped the
// status to consumed, ModifiedCount will be 0 and we skip the quota increment.
func (s *SyncHandler) finalizeReservationTransaction(ctx context.Context, res models.QuotaReservation, actualSize int64, userID string) error {
	reservations := s.DB.Collection("reservations")
	users := s.DB.Collection("users_data")

	if err := s.acquireWorker(ctx); err != nil {
		return err
	}
	defer s.releaseWorker()

	session, err := s.DB.Client().StartSession()
	if err != nil {
		return fmt.Errorf("start mongo session: %w", err)
	}
	defer session.EndSession(ctx)
	userObjectID, err := bson.ObjectIDFromHex(userID)
	if err != nil {
		return middleware.BadRequest("invalid user id")
	}
	_, err = session.WithTransaction(ctx, func(sc context.Context) (any, error) {
		// Guard: only consume if still pending. If manageReservationAndQuota
		// already won the race, ModifiedCount == 0 and we bail out cleanly.
		upd, err := reservations.UpdateOne(
			sc,
			bson.M{
				"_id":    res.ID,
				"status": string(ReservationPending),
			},
			bson.M{
				"$set": bson.M{
					"status":      string(ReservationConsumed),
					"consumed_at": time.Now(),
				},
			},
		)
		if err != nil {
			return nil, err
		}
		if upd.ModifiedCount == 0 {
			// Already handled by another caller.
			return nil, nil
		}

		// Increment the user's consumed quota. This matches the field name
		// in your User struct: QuotaBytes with bson tag "quota_used_bytes".
		_, err = users.UpdateOne(
			sc,
			bson.M{"_id": userObjectID},
			bson.M{
				"$inc": bson.M{
					"quota_used_bytes": actualSize,
				},
			},
		)
		if err != nil {
			return nil, err
		}

		return nil, nil
	})

	if err != nil {
		return fmt.Errorf("finalize reservation transaction: %w", err)
	}

	log.Debugw("worker finalized expired reservation",
		"attachment_id", res.AttachmentID,
		"user_id", res.UserID,
		"size", actualSize)
	return nil
}

// deleteS3ObjectSilent attempts to delete an S3 object and logs any error
// without propagating it, so that a single bad key does not break a batch.
func (s *SyncHandler) deleteS3ObjectSilent(ctx context.Context, key string) {
	if err := s.acquireWorker(ctx); err != nil {
		log.Errorw("acquire worker for silent S3 delete failed", "error", err)
		return
	}
	defer s.releaseWorker()

	_, err := s.s3Client.DeleteObject(ctx, &s3.DeleteObjectInput{
		Bucket: aws.String(s.s3Bucket), // TODO manage reservations nie ustawia na consumed i po zmianie online użytkownika lokalnie, notatki z mongo_id sie nie uploadują tylko upadtują, tylko to do naprawienia

		Key: aws.String(key),
	})
	if err != nil {
		log.Errorw("silent S3 delete failed", "key", key, "error", err)
	}
}

// ## TODO — Sync & Quota Debugging

// ### 4. Test failed / interrupted uploads

// * [ ] Create reservation and do not upload anything.
// * [ ] Let reservation expire.
// * [ ] Verify cleanup removes the reservation when S3 object does not exist.
// * [ ] Upload through presigned URL but do not call `/complete`.
// * [ ] Verify cleanup finds the object and consumes the reservation.
// * [ ] Upload an object with an incorrect size and verify it is removed.
// * [ ] Make `/complete` fail temporarily and verify the cleanup worker eventually fixes the state.

// ### 6. Cleanup worker

// * [ ] Verify worker starts at application startup.
// * [ ] Verify first cleanup runs immediately.
// * [ ] Verify periodic cleanup runs every 5 minutes.
// * [ ] Check logs for expired reservations and S3 reconciliation.
// * [ ] Verify worker does not delete a reservation when S3 temporarily returns a non-404 error.

// ### 7. Final sanity test

// * [ ] Fresh account.
// * [ ] New note.
// * [ ] Multiple attachments.
// * [ ] Quota reached.
// * [ ] Restart application.
// * [ ] Run sync again.
// * [ ] Verify MongoDB `quota_used_bytes`, reservations and S3 objects are all consistent.
// TODO manage reservations nie ustawia na consumed i po zmianie online użytkownika lokalnie, notatki z mongo_id sie nie uploadują tylko upadtują, tylko to do naprawienia
