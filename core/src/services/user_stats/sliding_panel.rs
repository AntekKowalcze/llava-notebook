//! # Sliding panel statistics module
//!
//! **Purpose**: This module collects the statistics and recently edited notes displayed in the
//! application's sliding panel.
//!
//! ## Exported items
//! * [`RecentlyEdited`] — Represents a recently edited note and the relative time since its last
//!   modification.
//! * [`BoxStats`] — Contains summary statistics displayed in the sliding panel.
//! * [`PanelData`] — Contains all data required by the sliding panel.
//! * [`get_sliding_panel_stats`] — Loads note statistics and recently edited notes from SQLite.
//!
//! ## Key design decisions
//! Deleted notes are excluded from all statistics and recently edited notes.
//!
//! Note titles are returned to the caller because they are required by the UI, but they are never
//! written to application logs.
//!
//! ## Dependencies
//! - `rusqlite` — Queries the local notes database
//! - `serde` — Serialises panel data for frontend communication
//! - `anyhow` — Adds context to database errors
//! - `tracing` — Logs operation status and database failures

use anyhow::Context;
use chacha20poly1305::Key;
use rusqlite::Connection;
use serde::Serialize;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RecentlyEdited {
    pub title: String,
    pub date: String,
    pub note_id: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BoxStats {
    pub number_of_notes: i64,
    pub favourites: i64,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PanelData {
    pub recently_edited: Vec<RecentlyEdited>,
    pub box_stats: BoxStats,
}

/// Loads statistics and recently edited notes for the sliding panel.
///
/// Deleted notes are excluded from the returned statistics. The function also converts the
/// modification timestamp of each recent note into a human-readable relative time.
///
/// # Errors
/// Returns an error if any SQLite query, statement preparation, row retrieval, or value conversion
/// fails.
pub fn get_sliding_panel_stats(
    user_id: &uuid::Uuid,
    notes_db: &Connection,
    notes_key: &Key,
) -> Result<PanelData, crate::errors::Error> {
    tracing::debug!(
        task = "getting sliding panel stats",
        %user_id,
        "starting sliding panel statistics collection"
    );

    let number_of_notes: i64 = notes_db
        .query_row(
            "SELECT COUNT(*)
             FROM notes
             WHERE deleted_at IS NULL",
            [],
            |row| row.get(0),
        )
        .inspect_err(|err| {
            tracing::error!(
                task = "getting sliding panel stats",
                status = "error",
                %user_id,
                error = ?err,
                "failed to get number of notes"
            );
        })
        .context("failed to get number of notes")?;

    let number_of_favourites: i64 = notes_db
        .query_row(
            "SELECT COUNT(*)
             FROM note_tags nt
             JOIN tags t ON nt.tag_id = t.tag_id
             JOIN notes n ON nt.note_local_id = n.local_id
             WHERE t.name = 'favourites'
               AND n.deleted_at IS NULL",
            [],
            |row| row.get(0),
        )
        .inspect_err(|err| {
            tracing::error!(
                task = "getting sliding panel stats",
                status = "error",
                %user_id,
                error = ?err,
                "failed to get number of favourite notes"
            );
        })
        .context("failed to get number of favourites")?;

    let box_stats = BoxStats {
        number_of_notes,
        favourites: number_of_favourites,
    };

    let mut stmt = notes_db
        .prepare(
            "SELECT title, updated_at, encrypted, local_id
             FROM notes
             WHERE deleted_at IS NULL
             ORDER BY updated_at DESC
             LIMIT 10",
        )
        .inspect_err(|err| {
            tracing::error!(
                task = "getting sliding panel stats",
                status = "error",
                %user_id,
                error = ?err,
                "failed to prepare recently edited notes query"
            );
        })
        .context("failed to prepare recently edited notes query")?;

    let mut rows = stmt
        .query([])
        .inspect_err(|err| {
            tracing::error!(
                task = "getting sliding panel stats",
                status = "error",
                %user_id,
                error = ?err,
                "failed to query recently edited notes"
            );
        })
        .context("failed to get last edited notes")?;

    let mut recently_edited = Vec::new();

    while let Some(row) = rows
        .next()
        .inspect_err(|err| {
            tracing::error!(
                task = "getting sliding panel stats",
                status = "error",
                %user_id,
                error = ?err,
                "failed to get next recently edited note row"
            );
        })
        .context("failed to get row")?
    {
        let title: String = row
            .get(0)
            .inspect_err(|err| {
                tracing::error!(
                    task = "getting sliding panel stats",
                    status = "error",
                    %user_id,
                    error = ?err,
                    "failed to get recently edited note title"
                );
            })
            .context("failed to get title")?;
        let is_encrypted: bool = row
            .get(2)
            .inspect_err(|err| {
                tracing::error!(
                    task = "getting sliding panel stats",
                    status = "error",
                    %user_id,
                    error = ?err,
                    "failed to get is encrypted"
                );
            })
            .context("failed to get title")?;
        let note_id: String = row
            .get(3)
            .inspect_err(|err| {
                tracing::error!(
                    task = "getting sliding panel stats",
                    status = "error",
                    %user_id,
                    error = ?err,
                    "failed to get local id"
                );
            })
            .context("failed to get title")?;

        let last_edited: i64 = row
            .get(1)
            .inspect_err(|err| {
                tracing::error!(
                    task = "getting sliding panel stats",
                    status = "error",
                    %user_id,
                    error = ?err,
                    "failed to get note updated_at"
                );
            })
            .context("failed to get updated_at")?;

        let edited_ago = crate::utils::get_time() - last_edited;
        let date = format_time_ago(edited_ago);
        if is_encrypted {
            let decrypted_title = crate::crypto::decrypt_title(&note_id, notes_key, notes_db)?;
            recently_edited.push(RecentlyEdited {
                title: decrypted_title,
                date,
                note_id,
            });
        } else {
            recently_edited.push(RecentlyEdited {
                title,
                date,
                note_id,
            });
        }
    }

    tracing::debug!(
        task = "getting sliding panel stats",
        status = "success",
        %user_id,
        number_of_notes,
        favourites = number_of_favourites,
        recently_edited = recently_edited.len(),
        "sliding panel statistics collected successfully"
    );

    Ok(PanelData {
        recently_edited,
        box_stats,
    })
}

/// Converts a duration in milliseconds into a human-readable relative time.
///
/// # Errors
/// This function does not return errors.
fn format_time_ago(milliseconds: i64) -> String {
    match milliseconds {
        0..=59_999 => "just now".to_string(),

        60_000..=3_599_999 => {
            let minutes = milliseconds / 60_000;

            return format!(
                "{} minute{} ago",
                minutes,
                if minutes == 1 { "" } else { "s" }
            )
        }

        3_600_000..=86_399_999 => {
            let hours = milliseconds / 3_600_000;

            return format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" })
        }

        86_400_000..=2_591_999_999 => {
            let days = milliseconds / 86_400_000;

           return format!("{} day{} ago", days, if days == 1 { "" } else { "s" })
        }

        2_592_000_000..=31_535_999_999 => {
            let weeks = milliseconds / 604_800_000;

           return format!("{} week{} ago", weeks, if weeks == 1 { "" } else { "s" })
        }

        _ => {
            let months = milliseconds / 2_592_000_000;

           return format!("{} month{} ago", months, if months == 1 { "" } else { "s" })
        }
    }
}
