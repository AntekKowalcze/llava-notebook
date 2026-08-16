//! # Dashboard statistics module
//!
//! **Purpose**: This module collects statistics used by the application dashboard from the local
//! notes and users databases.
//!
//! ## Exported items
//! * [`ActivityRecord`] — Represents the number of note edits performed on a particular day.
//! * [`DashboardData`] — Contains all statistics required to render the dashboard.
//! * [`get_dashboard_stats`] — Queries note, activity, account, and tag data and returns a
//!   [`DashboardData`] structure.
//!
//! ## Key design decisions
//! Dashboard statistics are calculated directly from the local SQLite databases. Deleted notes
//! are excluded from note and tag statistics.
//!
//! Only metadata required for diagnostics is written to logs. Note titles, tag names, and other
//! user-generated content are never logged.
//!
//! ## Dependencies
//! - `rusqlite` — Queries the local notes and users databases
//! - `serde` — Serialises dashboard structures for frontend communication
//! - `anyhow` — Adds context to database errors
//! - `tracing` — Logs dashboard operation status and database failures

use anyhow::Context;
use chacha20poly1305::Key;
use rusqlite::{named_params, Connection};
use serde::Serialize;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ActivityRecord {
    pub number_of_editions: i64,
    pub date: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DashboardData {
    pub number_of_notes: i64,
    pub number_of_encrypted_notes: i64,
    pub account_creation: i64,
    pub activity_vec: Vec<ActivityRecord>,
    pub last_three_edited: Vec<(String, String, String)>,
    pub favourite_tags: Vec<(String, String)>
}

/// Collects all statistics required by the application dashboard.
///
/// The function reads the number of notes, encrypted notes, account creation timestamp,
/// yearly activity, recently edited notes, and most frequently used tags.
///
/// # Errors
/// Returns an error if any database query or row conversion fails.
pub fn get_dashboard_stats(
    user_uuid: String,
    notes_db: &Connection,
    users_db: &Connection,
    notes_key: &Key
) -> Result<DashboardData, crate::errors::Error> {
    tracing::debug!(
        task = "getting dashboard stats",
        %user_uuid,
        "starting dashboard statistics collection"
    );

    let number_of_notes: i64 = notes_db
        .query_row(
            "SELECT COUNT(*) FROM notes WHERE is_deleted = 0",
            [],
            |row| row.get(0),
        )
        .inspect_err(|err| {
            tracing::error!(
                task = "getting dashboard stats",
                status = "error",
                error = ?err,
                %user_uuid,
                "failed to get number of notes"
            );
        })
        .context("failed to get number of notes")?;

    let number_of_encrypted_notes: i64 = notes_db
        .query_row(
            "SELECT COUNT(*)
             FROM notes
             WHERE encrypted = 1
             AND is_deleted = 0",
            [],
            |row| row.get(0),
        )
        .inspect_err(|err| {
            tracing::error!(
                task = "getting dashboard stats",
                status = "error",
                error = ?err,
                %user_uuid,
                "failed to get number of encrypted notes"
            );
        })
        .context("failed to get number of encrypted notes")?;

    let account_creation: i64 = users_db
        .query_row(
            "SELECT created_at
             FROM users_data
             WHERE user_id = :id",
            named_params! {
                ":id": user_uuid,
            },
            |row| row.get(0),
        )
        .inspect_err(|err| {
            tracing::error!(
                task = "getting dashboard stats",
                status = "error",
                error = ?err,
                %user_uuid,
                "failed to get account creation date"
            );
        })
        .context("failed to get account creation date")?;

    let mut activity_vec = Vec::new();

    let mut stmt = notes_db
        .prepare(
            "SELECT
                DATE(date) AS day,
                COUNT(*) AS edits
             FROM user_activity
             WHERE date >= DATE('now', '-1 year')
             GROUP BY DATE(date)
             ORDER BY day",
        )
        .inspect_err(|err| {
            tracing::error!(
                task = "getting dashboard stats",
                status = "error",
                error = ?err,
                %user_uuid,
                "failed to prepare activity query"
            );
        })
        .context("failed to prepare activity query")?;

    let mut rows = stmt
        .query([])
        .inspect_err(|err| {
            tracing::error!(
                task = "getting dashboard stats",
                status = "error",
                error = ?err,
                %user_uuid,
                "failed to query activity data"
            );
        })
        .context("failed to query activity data")?;

    while let Some(row) = rows.next().context("failed to get next activity row")? {
        let date: String = row
            .get(0)
            .inspect_err(|err| {
                tracing::error!(
                    task = "getting dashboard stats",
                    status = "error",
                    error = ?err,
                    %user_uuid,
                    "failed to get activity date"
                );
            })
            .context("failed to get activity date")?;

        let number_of_editions: i64 = row
            .get(1)
            .inspect_err(|err| {
                tracing::error!(
                    task = "getting dashboard stats",
                    status = "error",
                    error = ?err,
                    %user_uuid,
                    "failed to get activity edit count"
                );
            })
            .context("failed to get activity edit count")?;

        activity_vec.push(ActivityRecord {
            number_of_editions,
            date,
        });
    }

    let mut last_three_edited = Vec::new();

    let mut stmt = notes_db
        .prepare(
            "SELECT
                note_id,
                MAX(datetime(date)) AS last_edit
             FROM user_activity
             WHERE note_id IS NOT NULL
             GROUP BY note_id
             ORDER BY last_edit DESC
             LIMIT 3",
        )
        .inspect_err(|err| {
            tracing::error!(
                task = "getting dashboard stats",
                status = "error",
                error = ?err,
                %user_uuid,
                "failed to prepare recent notes query"
            );
        })
        .context("failed to prepare recent notes query")?;

    let mut rows = stmt
        .query([])
        .inspect_err(|err| {
            tracing::error!(
                task = "getting dashboard stats",
                status = "error",
                error = ?err,
                %user_uuid,
                "failed to query recent notes"
            );
        })
        .context("failed to query recent notes")?;

    while let Some(row) = rows
        .next()
        .context("failed to get next recent note row")?
    {
        let note_id: String = row
            .get(0)
            .inspect_err(|err| {
                tracing::error!(
                    task = "getting dashboard stats",
                    status = "error",
                    error = ?err,
                    %user_uuid,
                    "failed to get recent note id"
                );
            })
            .context("failed to get recent note id")?;

        let datetime: String = row
            .get(1)
            .inspect_err(|err| {
                tracing::error!(
                    task = "getting dashboard stats",
                    status = "error",
                    error = ?err,
                    %user_uuid,
                    "failed to get recent note date"
                );
            })
            .context("failed to get recent note date")?;

        let (title, encrypted): (String,bool) = notes_db
            .query_row(
                "SELECT title, encrypted
                 FROM notes
                 WHERE local_id = :note_id",
                named_params! {
                    ":note_id": note_id,
                },
                |row|  Ok((row.get(0)?, row.get(1)?)),
            )
            .inspect_err(|err| {
                tracing::error!(
                    task = "getting dashboard stats",
                    status = "error",
                    error = ?err,
                    %user_uuid,
                    "failed to get recent note title"
                );
            })
            .context("failed to get recent note title")?;
            if encrypted {
               let decrypted_title = crate::crypto::decrypt_title(&note_id, notes_key, notes_db)?;
                  last_three_edited.push((decrypted_title, note_id, datetime));
            }else{
                  last_three_edited.push((title, note_id, datetime));
            }



      
    }

    let mut favourite_tags = Vec::new();

    let mut stmt = notes_db
        .prepare(
            "SELECT
                t.name,
                t.color,
                COUNT(nt.note_local_id) AS usage_count
             FROM tags t
             JOIN note_tags nt
                ON nt.tag_id = t.tag_id
             JOIN notes n
                ON n.local_id = nt.note_local_id
                AND n.is_deleted = 0
             GROUP BY t.tag_id, t.name, t.color
             ORDER BY usage_count DESC
             LIMIT 3",
        )
        .inspect_err(|err| {
            tracing::error!(
                task = "getting dashboard stats",
                status = "error",
                error = ?err,
                %user_uuid,
                "failed to prepare favourite tags query"
            );
        })
        .context("failed to prepare favourite tags query")?;

    let mut rows = stmt
        .query([])
        .inspect_err(|err| {
            tracing::error!(
                task = "getting dashboard stats",
                status = "error",
                error = ?err,
                %user_uuid,
                "failed to query favourite tags"
            );
        })
        .context("failed to query favourite tags")?;

    while let Some(row) = rows
        .next()
        .context("failed to get next favourite tag row")?
    {
        let tag_name: String = row
            .get(0)
            .inspect_err(|err| {
                tracing::error!(
                    task = "getting dashboard stats",
                    status = "error",
                    error = ?err,
                    %user_uuid,
                    "failed to get favourite tag name"
                );
            })
            .context("failed to get favourite tag name")?;

        let tag_colour: String = row
            .get(1)
            .inspect_err(|err| {
                tracing::error!(
                    task = "getting dashboard stats",
                    status = "error",
                    error = ?err,
                    %user_uuid,
                    "failed to get favourite tag colour"
                );
            })
            .context("failed to get favourite tag colour")?;

        favourite_tags.push((tag_name, tag_colour));
    }

    tracing::debug!(
        task = "getting dashboard stats",
        status = "success",
        %user_uuid,
        number_of_notes,
        number_of_encrypted_notes,
        activity_days = activity_vec.len(),
        recent_notes = last_three_edited.len(),
        favourite_tags = favourite_tags.len(),
        "dashboard statistics collected successfully"
    );

    Ok(DashboardData {
        number_of_notes,
        number_of_encrypted_notes,
        account_creation,
        activity_vec,
        last_three_edited,
        favourite_tags,
    })
}