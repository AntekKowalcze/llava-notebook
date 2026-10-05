package routes

import (
	"context"
	"net/http"
	"net/http/httptest"
	"os"
	"strconv"
	"strings"
	"sync/atomic"
	"testing"
	"time"

	"llava-server/middleware"
	"llava-server/models"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/credentials"
	"github.com/aws/aws-sdk-go-v2/service/s3"
	"github.com/go-playground/validator/v10"
	"github.com/gofiber/fiber/v3"
	"github.com/google/uuid"
	"go.mongodb.org/mongo-driver/v2/bson"
	"go.mongodb.org/mongo-driver/v2/mongo"
	"go.mongodb.org/mongo-driver/v2/mongo/options"
)

// These tests need a real MongoDB (hard delete uses a pipeline update).
// Point MONGO_TEST_URI at a throwaway instance, e.g.
//
//	docker run -d -p 27099:27017 mongo:7 --replSet rs0
//	MONGO_TEST_URI='mongodb://localhost:27099/?directConnection=true'
func newTestSyncHandler(t *testing.T) *SyncHandler {
	t.Helper()

	uri := os.Getenv("MONGO_TEST_URI")
	if uri == "" {
		t.Skip("MONGO_TEST_URI not set")
	}

	client, err := mongo.Connect(options.Client().ApplyURI(uri))
	if err != nil {
		t.Fatalf("connect: %v", err)
	}

	dbName := "llava_test_" + bson.NewObjectID().Hex()
	db := client.Database(dbName)

	t.Cleanup(func() {
		ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cancel()
		_ = db.Drop(ctx)
		_ = client.Disconnect(ctx)
	})

	s := NewSyncHandler(db, validator.New(), nil, "", nil)

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	if err := s.EnsureIndexes(ctx); err != nil {
		t.Fatalf("ensure indexes: %v", err)
	}

	return s
}

func insertNote(t *testing.T, s *SyncHandler, owner, localID string, version int64) bson.ObjectID {
	t.Helper()

	id := bson.NewObjectID()
	_, err := s.Coll.InsertOne(context.Background(), NoteDocument{
		ID:           id,
		OwnerID:      owner,
		LocalID:      localID,
		CloudVersion: version,
		Title:        "title",
		Content:      "content",
	})
	if err != nil {
		t.Fatalf("insert note: %v", err)
	}

	return id
}

func TestHardDeleteKeepsLocalIDSoUploadCannotResurrect(t *testing.T) {
	s := newTestSyncHandler(t)
	ctx := context.Background()

	id := insertNote(t, s, "user1", "local-1", 3)

	tombstone, err := hardDeleteNoteInCloud(ctx, s, "user1", id, 3, nil)
	if err != nil {
		t.Fatalf("hard delete: %v", err)
	}
	if tombstone.CloudVersion != 4 {
		t.Fatalf("tombstone version = %d, want 4", tombstone.CloudVersion)
	}

	var stored NoteDocument
	if err := s.Coll.FindOne(ctx, bson.M{"_id": id}).Decode(&stored); err != nil {
		t.Fatalf("load tombstone: %v", err)
	}
	if !stored.HardDeleted || stored.LocalID != "local-1" || stored.Content != "" {
		t.Fatalf("unexpected tombstone: %+v", stored)
	}

	// A retried upload for the same local ID must hit the tombstone, not
	// create a second live note.
	var again NoteDocument
	err = s.Coll.FindOneAndUpdate(
		ctx,
		bson.M{"owner_id": "user1", "local_id": "local-1"},
		bson.M{"$setOnInsert": NoteDocument{OwnerID: "user1", LocalID: "local-1", CloudVersion: 1}},
		options.FindOneAndUpdate().SetUpsert(true).SetReturnDocument(options.After),
	).Decode(&again)
	if err != nil {
		t.Fatalf("upsert: %v", err)
	}
	if again.ID != id || !again.HardDeleted {
		t.Fatalf("upload resurrected the note: %+v", again)
	}

	n, _ := s.Coll.CountDocuments(ctx, bson.M{"owner_id": "user1"})
	if n != 1 {
		t.Fatalf("expected 1 document, got %d", n)
	}
}

func TestHardDeleteRejectsStaleVersion(t *testing.T) {
	s := newTestSyncHandler(t)
	id := insertNote(t, s, "user1", "local-1", 5)

	if _, err := hardDeleteNoteInCloud(context.Background(), s, "user1", id, 4, nil); err == nil {
		t.Fatal("expected stale hard delete to fail")
	}
}

func TestClientHardDeleteWithoutCloudIDRemovesOrphanedCloudNote(t *testing.T) {
	s := newTestSyncHandler(t)
	ctx := context.Background()

	// Upload succeeded server-side, but the response was lost, so the client
	// never learned the cloud ID.
	id := insertNote(t, s, "user1", "local-1", 1)

	result, err := checkNoteSync(models.CheckNoteSyncStatus{
		LocalID:     "local-1",
		SyncState:   "WaitingForTombstone",
		HardDeleted: true,
	}, ctx, "user1", s)
	if err != nil {
		t.Fatalf("checkNoteSync: %v", err)
	}
	if len(result.NotesToHardDelete) != 1 {
		t.Fatalf("expected client to purge its tombstone row, got %+v", result)
	}

	var stored NoteDocument
	if err := s.Coll.FindOne(ctx, bson.M{"_id": id}).Decode(&stored); err != nil {
		t.Fatalf("load note: %v", err)
	}
	if !stored.HardDeleted {
		t.Fatalf("cloud note was left alive: %+v", stored)
	}

	// Never uploaded at all: the client is still told to purge its row.
	result, err = checkNoteSync(models.CheckNoteSyncStatus{
		LocalID:     "never-uploaded",
		SyncState:   "WaitingForTombstone",
		HardDeleted: true,
	}, ctx, "user1", s)
	if err != nil || len(result.NotesToHardDelete) != 1 {
		t.Fatalf("never-uploaded note: result=%+v err=%v", result, err)
	}
}

func TestClientVersionAheadOfServerIsReportedAsFailure(t *testing.T) {
	s := newTestSyncHandler(t)
	id := insertNote(t, s, "user1", "local-1", 2)
	clientVersion := int64(7)

	result, err := checkNoteSync(models.CheckNoteSyncStatus{
		LocalID:      "local-1",
		SyncState:    "PendingUpload",
		CloudID:      &id,
		CloudVersion: &clientVersion,
	}, context.Background(), "user1", s)
	if err == nil {
		t.Fatalf("expected an error, got result %+v", result)
	}
}

func TestFullSyncDoesNotRedownloadNoteWhoseUploadResponseWasLost(t *testing.T) {
	s := newTestSyncHandler(t)
	insertNote(t, s, "user1", "local-1", 1)

	// The client knows the note by local ID only (no cloud ID yet).
	notes, attachments, err := checkNotesNotExistingOnClient(
		s,
		context.Background(),
		"user1",
		[]models.CheckNoteSyncStatus{{LocalID: "local-1", SyncState: "PendingUpload"}},
	)
	if err != nil {
		t.Fatalf("full sync: %v", err)
	}
	if len(notes) != 0 || len(attachments) != 0 {
		t.Fatalf("note would be downloaded as a duplicate: %+v", notes)
	}
}

// fakeS3 answers HeadObject/DeleteObject for a single object and records
// deletes, which is all the upload-complete handler needs.
type fakeS3 struct {
	size     int64
	metadata map[string]string
	deleted  atomic.Int32
}

func (f *fakeS3) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	switch r.Method {
	case http.MethodHead:
		w.Header().Set("Content-Length", strconv.FormatInt(f.size, 10))
		for k, v := range f.metadata {
			w.Header().Set("x-amz-meta-"+k, v)
		}
		w.WriteHeader(http.StatusOK)
	case http.MethodDelete:
		f.deleted.Add(1)
		w.WriteHeader(http.StatusNoContent)
	default:
		w.WriteHeader(http.StatusNotImplemented)
	}
}

func newUploadCompleteFixture(t *testing.T, fake *fakeS3) (*SyncHandler, *fiber.App, string) {
	t.Helper()

	s := newTestSyncHandler(t)
	srv := httptest.NewServer(fake)
	t.Cleanup(srv.Close)

	s.s3Client = s3.New(s3.Options{
		Region:       "us-east-1",
		BaseEndpoint: aws.String(srv.URL),
		UsePathStyle: true,
		Credentials:  credentials.NewStaticCredentialsProvider("k", "s", ""),
	})
	s.s3Bucket = "bucket"

	userID := bson.NewObjectID().Hex()
	userObjectID, _ := bson.ObjectIDFromHex(userID)
	if _, err := s.DB.Collection("users_data").InsertOne(context.Background(), bson.M{
		"_id": userObjectID, "quota_reserved_bytes": int64(100), "quota_used_bytes": int64(0),
	}); err != nil {
		t.Fatalf("insert user: %v", err)
	}

	app := fiber.New(fiber.Config{ErrorHandler: middleware.ErrorHandler})
	app.Post("/upload-compleated/:attachment_id", func(c fiber.Ctx) error {
		c.Locals("userID", userID)
		return s.manageReservationAndQuota(c)
	})

	return s, app, userID
}

func insertReservation(t *testing.T, s *SyncHandler, userID, attachmentID string, size int64, expires time.Time) {
	t.Helper()
	_, err := s.DB.Collection("reservations").InsertOne(context.Background(), models.QuotaReservation{
		UserID: userID, AttachmentID: attachmentID, SizeBytes: size,
		Status: string(ReservationPending), ExpiresAt: expires, CreatedAt: time.Now(),
	})
	if err != nil {
		t.Fatalf("insert reservation: %v", err)
	}
}

func userQuota(t *testing.T, s *SyncHandler, userID string) (used, reserved int64) {
	t.Helper()
	oid, _ := bson.ObjectIDFromHex(userID)
	var u struct {
		Used     int64 `bson:"quota_used_bytes"`
		Reserved int64 `bson:"quota_reserved_bytes"`
	}
	if err := s.DB.Collection("users_data").FindOne(context.Background(), bson.M{"_id": oid}).Decode(&u); err != nil {
		t.Fatalf("load user: %v", err)
	}
	return u.Used, u.Reserved
}

func postComplete(t *testing.T, app *fiber.App, attachmentID string) int {
	t.Helper()
	resp, err := app.Test(httptest.NewRequest(http.MethodPost, "/upload-compleated/"+attachmentID, nil))
	if err != nil {
		t.Fatalf("request: %v", err)
	}
	defer resp.Body.Close()
	return resp.StatusCode
}

func TestUploadCompleteChargesExpiredButUploadedReservation(t *testing.T) {
	fake := &fakeS3{size: 40}
	s, app, userID := newUploadCompleteFixture(t, fake)
	attachmentID := uuid.NewString()
	insertReservation(t, s, userID, attachmentID, 40, time.Now().Add(-time.Minute))

	if code := postComplete(t, app, attachmentID); code != http.StatusOK {
		t.Fatalf("status = %d, want 200", code)
	}
	if used, reserved := userQuota(t, s, userID); used != 40 || reserved != 60 {
		t.Fatalf("quota used=%d reserved=%d, want 40/60", used, reserved)
	}
	if fake.deleted.Load() != 0 {
		t.Fatal("a charged upload must not be deleted")
	}

	// A retried completion is a no-op, not a double charge.
	if code := postComplete(t, app, attachmentID); code != http.StatusOK {
		t.Fatalf("retry status = %d, want 200", code)
	}
	if used, _ := userQuota(t, s, userID); used != 40 {
		t.Fatalf("retry changed used quota to %d", used)
	}
}

func TestUploadCompleteDeletesObjectThatWasNeverReserved(t *testing.T) {
	fake := &fakeS3{size: 40}
	_, app, _ := newUploadCompleteFixture(t, fake)

	if code := postComplete(t, app, uuid.NewString()); code != http.StatusConflict {
		t.Fatalf("status = %d, want 409", code)
	}
	if fake.deleted.Load() != 1 {
		t.Fatalf("unreserved object was not deleted (deletes=%d)", fake.deleted.Load())
	}
}

func TestUploadCompleteDiscardsUploadForHardDeletedNote(t *testing.T) {
	noteID := bson.NewObjectID()
	fake := &fakeS3{size: 40, metadata: map[string]string{"note_cloud_id": noteID.Hex()}}
	s, app, userID := newUploadCompleteFixture(t, fake)

	if _, err := s.Coll.InsertOne(context.Background(), NoteDocument{
		ID: noteID, OwnerID: userID, LocalID: "l", CloudVersion: 2, HardDeleted: true,
	}); err != nil {
		t.Fatalf("insert tombstone: %v", err)
	}
	attachmentID := uuid.NewString()
	insertReservation(t, s, userID, attachmentID, 40, time.Now().Add(time.Minute))

	if code := postComplete(t, app, attachmentID); code != http.StatusConflict {
		t.Fatalf("status = %d, want 409", code)
	}
	if fake.deleted.Load() != 1 {
		t.Fatalf("object for deleted note was kept (deletes=%d)", fake.deleted.Load())
	}
	if used, reserved := userQuota(t, s, userID); used != 0 || reserved != 60 {
		t.Fatalf("quota used=%d reserved=%d, want 0/60 (reservation released)", used, reserved)
	}
}

func TestErrorStateNoteIsReuploadedNotReportedSynced(t *testing.T) {
	s := newTestSyncHandler(t)
	id := insertNote(t, s, "user1", "local-1", 2)
	version := int64(2)

	result, err := checkNoteSync(models.CheckNoteSyncStatus{
		LocalID: "local-1", SyncState: "Error", CloudID: &id, CloudVersion: &version,
	}, context.Background(), "user1", s)
	if err != nil {
		t.Fatalf("checkNoteSync: %v", err)
	}
	if len(result.NotesToUpload) != 1 || len(result.SyncedNotes) != 0 {
		t.Fatalf("Error note must be retried, got %+v", result)
	}
}

func TestSyncOffersAttachmentAddedByAnotherDevice(t *testing.T) {
	noteID := bson.NewObjectID()
	attachmentID := uuid.NewString()
	fake := &fakeS3{size: 5, metadata: map[string]string{
		"checksum": "abc", "size_bytes": "5", "is_encrypted": "false",
		"file_name": "pic.png", "mime_type": "image/png",
		"note_cloud_id": noteID.Hex(), "created_at": "1", "updated_at": "2",
	}}
	s, _, userID := newUploadCompleteFixture(t, fake)
	s.presigner = s3.NewPresignClient(s.s3Client)

	// The note exists on both devices at the same version; another device
	// attached a picture this client has never heard of.
	if _, err := s.Coll.InsertOne(context.Background(), bson.M{
		"_id": noteID, "owner_id": userID, "local_id": "l", "cloud_version": int64(3),
		"attachment_ids": bson.A{attachmentID},
	}); err != nil {
		t.Fatalf("insert note: %v", err)
	}
	version := int64(3)

	result, err := checkNoteSync(models.CheckNoteSyncStatus{
		LocalID: "l", SyncState: "Synced", CloudID: &noteID, CloudVersion: &version,
	}, context.Background(), userID, s)
	if err != nil {
		t.Fatalf("checkNoteSync: %v", err)
	}
	if len(result.AttachmentsToDownload) != 1 ||
		result.AttachmentsToDownload[0].AttachmentID.String() != attachmentID ||
		result.AttachmentsToDownload[0].DownloadUrl == "" {
		t.Fatalf("attachment not offered: %+v", result.AttachmentsToDownload)
	}

	// Once the client has it, nothing more is offered.
	result, err = checkNoteSync(models.CheckNoteSyncStatus{
		LocalID: "l", SyncState: "Synced", CloudID: &noteID, CloudVersion: &version,
		Attachments: []models.AttachmentSyncCheck{{
			AttachmentID: uuid.MustParse(attachmentID), ChecksumEncrypted: "abc",
			FileName: "pic.png", MimeType: "image/png",
		}},
	}, context.Background(), userID, s)
	if err != nil {
		t.Fatalf("checkNoteSync: %v", err)
	}
	for _, d := range result.AttachmentsToDownload {
		t.Fatalf("unexpected download once client has the attachment: %+v", d)
	}
}

func TestDummySaltIsStablePerEmailAndMatchesClientSaltFormat(t *testing.T) {
	t.Setenv("AUTH_HMAC_SECRET", "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff")

	a1, a2 := dummySalt("a@example.com"), dummySalt("a@example.com")
	if a1 != a2 {
		t.Fatalf("salt must be stable for one email: %q vs %q", a1, a2)
	}
	// Lookups are case-sensitive, so a differently cased email is a
	// different account and must get its own salt. A shared salt would make
	// unregistered emails answer identically to both spellings while a
	// registered one does not, revealing which emails exist.
	if dummySalt("A@Example.com") == a1 {
		t.Fatal("a differently cased email must get its own salt")
	}
	if dummySalt("b@example.com") == a1 {
		t.Fatal("different emails must get different salts")
	}
	// argon2 SaltString for 16 bytes is 22 unpadded base64 characters.
	if len(a1) != 22 || strings.ContainsAny(a1, "=") {
		t.Fatalf("unexpected salt format %q", a1)
	}
}

func TestQueryIndexesAreCreated(t *testing.T) {
	s := newTestSyncHandler(t)
	ctx := context.Background()

	for _, c := range []struct{ coll, index string }{
		{"notes", "owner_id"},
		{"reservations", "status_expires_at"},
		{"reservations", "user_attachment"},
	} {
		cursor, err := s.DB.Collection(c.coll).Indexes().List(ctx)
		if err != nil {
			t.Fatalf("list indexes: %v", err)
		}
		var found bool
		var idx []bson.M
		if err := cursor.All(ctx, &idx); err != nil {
			t.Fatalf("read indexes: %v", err)
		}
		for _, i := range idx {
			if i["name"] == c.index {
				found = true
			}
		}
		if !found {
			t.Fatalf("index %s.%s missing", c.coll, c.index)
		}
	}
}
