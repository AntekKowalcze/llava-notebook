//! # Local tag management module
//!
//! **Purpose**: This module is responsible for creating, validating, assigning,
//! retrieving, and removing tags stored in the local SQLite database.
//!
//! It manages both tag definitions and the many-to-many relationships between
//! notes and tags. Tags are scoped to their owner, allowing each local user to
//! maintain an independent set of tags.
//!
//! ## Exports
//!
//! * [`add_tag_to_database`] — Creates a new tag for a user after validating
//!   its name and returns the generated tag identifier.
//! * [`add_tag_to_note`] — Associates an existing tag with a local note.
//! * [`remove_tag_from_note`] — Removes a tag association from a local note.
//! * [`verify_tag_name`] — Validates a tag name and checks whether the user
//!   already has a tag with the same name.
//! * [`remove_tag`] — Permanently removes a tag from the local database.
//! * [`get_all_tags_for_note`] — Retrieves all tags assigned to a note,
//!   ordered alphabetically by name.
//! * [`UiTag`] — Serializable representation of a tag used when returning tag
//!   data to the user interface.
//! * [`get_all_tags`] — Retrieves all tags belonging to a user, ordered
//!   alphabetically by name.
//! * [`find_tag_id`] — Finds a user's tag identifier by name without returning
//!   an error when the tag does not exist.
//!
//! ## Key design decisions
//!
//! Tags are scoped by `owner_id`. This prevents tags belonging to different
//! users from being treated as the same tag and allows every user to maintain
//! an independent tag namespace.
//!
//! Tag names are validated before insertion. Leading and trailing whitespace
//! is ignored for validation purposes, empty names are rejected, and names are
//! limited to 50 bytes. A user cannot create multiple tags with the same name.
//!
//! Tags and note-tag relationships are stored separately. The `note_tags`
//! table represents the many-to-many relationship between notes and tags,
//! allowing one tag to be assigned to multiple notes and one note to contain
//! multiple tags.
//!
//! Tag identifiers are generated as UUIDv4 values, providing unique
//! identifiers independent of the SQLite database row or tag name.
//!
//! [`UiTag`] is kept separate from the database representation so that only
//! the fields required by the user interface are exposed to the frontend.
//!
//! ## Dependencies
//!
//! * [`rusqlite`] — Local SQLite access for tag and note-tag persistence.
//! * [`uuid`] — UUIDv4 generation for new tag identifiers.
//! * [`serde`] — Serialization of [`UiTag`] for communication with the user
//!   interface.
//! * [`anyhow`] — Adds context to database operation failures before they are
//!   returned as application errors.
//! * [`crate::utils`] — Provides timestamp generation for newly created tag
//!   and note-tag records.
//! * [`crate::errors`] — Application-level errors such as invalid tag names,
//!   duplicate tags, and database failures.

use anyhow::Context;
use rusqlite::OptionalExtension;
use rusqlite::named_params;
use serde::Serialize;
use uuid::Uuid;

pub fn add_tag_to_database(
    notes_db: &rusqlite::Connection,
    tag_name: String,
    tag_color: String,
    owner_id: &str,
) -> Result<String, crate::errors::Error> {
    verify_tag_name(owner_id, &tag_name, notes_db)?;

    let tag_id = Uuid::new_v4().to_string();
    let created_at = crate::utils::get_time();

    notes_db
        .execute(
            "INSERT INTO tags (
                tag_id,
                owner_id,
                name,
                color,
                created_at
            ) VALUES (
                :tag_id,
                :owner_id,
                :name,
                :color,
                :created_at
            )",
            named_params! {
                ":tag_id": tag_id,
                ":owner_id": owner_id,
                ":name": tag_name,
                ":color": tag_color,
                ":created_at": created_at,
            },
        )
        .context("failed to add tag to database")?;

    Ok(tag_id.to_string())
}

pub fn add_tag_to_note(
    notes_db: &rusqlite::Connection,
    note_id: String,
    tag_id: String,
) -> Result<(), crate::errors::Error> {
    let created_at = crate::utils::get_time();

    notes_db
        .execute(
            "INSERT INTO note_tags (
                note_local_id,
                tag_id,
                created_at
            ) VALUES (
                :note_id,
                :tag_id,
                :created_at
            )",
            named_params! {
                ":note_id": note_id,
                ":tag_id": tag_id,
                ":created_at": created_at,
            },
        )
        .context("failed to add tag to note")?;

    Ok(())
}

pub fn remove_tag_from_note(
    notes_db: &rusqlite::Connection,
    note_id: &str,
    tag_id: &str,
) -> Result<(), crate::errors::Error> {
    notes_db
        .execute(
            "DELETE FROM note_tags
             WHERE note_local_id = :note_id
             AND tag_id = :tag_id",
            rusqlite::named_params! {
                ":note_id": note_id,
                ":tag_id": tag_id,
            },
        )
        .context("failed to remove tag from note")?;

    Ok(())
}
/// Validates a tag name and checks whether it is already used by the owner.
///
/// Leading and trailing whitespace is ignored when validating the name. Empty
/// names and names longer than 50 bytes are rejected before checking the
/// database for duplicates.
///
/// # Errors
///
/// Returns [`crate::errors::Error::InvalidTagName`] when the trimmed tag name
/// is empty or exceeds the maximum allowed length.
///
/// Returns [`crate::errors::Error::TagAlreadyExists`] when the owner already
/// has a tag with the same name.
///
/// Returns an application error when the database query fails.
pub fn verify_tag_name(
    owner_id: &str,
    tag_name: &str,
    notes_db: &rusqlite::Connection,
) -> Result<(), crate::errors::Error> {
    let tag_name = tag_name.trim();

    if tag_name.is_empty() || tag_name.len() > 50 {
        return Err(crate::errors::Error::InvalidTagName);
    }

    let exists: bool = notes_db
        .query_row(
            "SELECT EXISTS(
                SELECT 1
                FROM tags
                WHERE owner_id = :owner_id
                  AND name = :name
            )",
            rusqlite::named_params! {
                ":owner_id": owner_id,
                ":name": tag_name,
            },
            |row| row.get(0),
        )
        .context("failed to verify tag name")?;

    if exists {
        return Err(crate::errors::Error::TagAlreadyExists);
    }

    Ok(())
}

pub fn remove_tag(
    notes_db: &rusqlite::Connection,
    tag_id: &str,
) -> Result<(), crate::errors::Error> {
    notes_db
        .execute(
            "DELETE FROM tags
             WHERE tag_id = :tag_id",
            rusqlite::named_params! {
                ":tag_id": tag_id,
            },
        )
        .context("failed to remove tag")?;

    Ok(())
}

pub fn get_all_tags_for_note(
    notes_db: &rusqlite::Connection,
    note_id: &str,
) -> Result<Vec<UiTag>, crate::errors::Error> {
    let mut stmt = notes_db
        .prepare(
            "SELECT t.tag_id, t.name, t.color
             FROM tags t
             JOIN note_tags nt ON nt.tag_id = t.tag_id
             WHERE nt.note_local_id = :note_id
             ORDER BY t.name",
        )
        .context("failed to prepare get tags query")?;

    let tags = stmt
        .query_map(
            named_params! {
                ":note_id": note_id,
            },
            |row| {
                Ok(UiTag {
                    tag_id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                })
            },
        )
        .context("failed to get tags for note")?
        .collect::<Result<Vec<UiTag>, _>>()
        .context("failed to read tags for note")?;

    Ok(tags)
}
#[derive(Debug, Serialize)]
pub struct UiTag {
    pub tag_id: String,
    pub name: String,
    pub color: String,
}

pub fn get_all_tags(
    notes_db: &rusqlite::Connection,
    owner_id: &str,
) -> Result<Vec<UiTag>, crate::errors::Error> {
    let mut stmt = notes_db
        .prepare(
            "SELECT tag_id, name, color
             FROM tags
             WHERE owner_id = :owner_id
             ORDER BY name",
        )
        .context("failed to prepare get all tags query")?;

    let tags = stmt
        .query_map(
            named_params! {
                ":owner_id": owner_id,
            },
            |row| {
                Ok(UiTag {
                    tag_id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                })
            },
        )
        .context("failed to get all tags")?
        .collect::<Result<Vec<UiTag>, _>>()
        .context("failed to read tags")?;

    Ok(tags)
}

pub fn find_tag_id(
    notes_db: &rusqlite::Connection,
    owner_id: &str,
    tag_name: &str,
) -> Result<Option<String>, crate::errors::Error> {
    let tag_id = notes_db
        .query_row(
            "SELECT tag_id
             FROM tags
             WHERE owner_id = :owner_id
               AND name = :name",
            named_params! {
                ":owner_id": owner_id,
                ":name": tag_name,
            },
            |row| row.get::<_, String>(0),
        )
        .optional()
        .context("failed to check if tag exists")?;

    Ok(tag_id)
}
