use anyhow::Context;
use rusqlite::Connection;
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
    pub last_three_edited: Vec<(String, String)>, //note_id, date
    pub favourite_tags: Vec<(String, String)>,    // tag_name, color
}
pub fn get_dashboard_stats(
    user_uuid: String,
    notes_db: &Connection,
    users_db: &Connection,
) -> Result<DashboardData, crate::errors::Error> {
    let number_of_notes: i64 = notes_db
        .query_row(
            "SELECT COUNT(*) FROM notes WHERE is_deleted = 0",
            [],
            |row| row.get(0),
        )
        .inspect_err(|err| {
            tracing::error!(task="getting dashboard stats",
                status = "error",
                error = ?err,
                %user_uuid,
                "failed to get number of notes"
            )
        })
        .context("Failed to get dashboard data from notes_db")?;
    let number_of_encrypted_notes: i64 = notes_db
        .query_row(
            "SELECT COUNT(*) FROM notes WHERE encrypted = 1 AND is_deleted = 0",
            [],
            |row| row.get(0),
        )
        .inspect_err(|err| {
            tracing::error!(task="getting dashboard stats",
                status = "error",
                error = ?err,
                %user_uuid,
                "failed to get number of ecrypted notes"
            )
        })
        .context("Failed to get dashboard data from notes_db")?;

    let account_creation = users_db
        .query_row(
            "SELECT created_at FROM users_data WHERE user_id = :id",
            rusqlite::named_params! {
                ":id": user_uuid
            },
            |row| row.get(0),
        )
        .inspect_err(|err| {
            tracing::error!(task="getting dashboard stats",
                status = "error",
                error = ?err,
                %user_uuid,
                "failed to get created_at account data"
            )
        })
        .context("Failed to get dashboard data from users_db")?;
    let mut activity_vec: Vec<ActivityRecord> = Vec::new();

    let mut stmt = notes_db.prepare("SELECT DATE(date) AS day, COUNT(*) AS edits FROM user_activity WHERE date >= DATE('now', '-1 year') GROUP BY DATE(date) ORDER BY day").context("Failed to prepare sql query")?;

    let mut handle = stmt
        .query([])
        .inspect_err(|err| {
            tracing::error!(task="getting dashboard stats",
                status = "error",
                error = ?err,
                %user_uuid,
                "failed to get activity data")
        })
        .context("Failed to get activity data")?;

    while let Some(row) = handle.next().context("failed to get next row")? {
        let date: String = row.get(0).inspect_err(|err| tracing::error!(task="getting dashboard stats", error=?err, %user_uuid, "Failed to get date")).context("Db error while getting dashboard data")?;
        let number_of_editions: i64 =  row.get(1).inspect_err(|err| tracing::error!(task="getting dashboard stats", error=?err, %user_uuid, "Failed to get number_of_editions")).context("Db error while getting dashboard data")?;
        activity_vec.push(ActivityRecord {
            number_of_editions,
            date,
        });
    }
    let mut last_three_edited: Vec<(String, String)> = Vec::new();
    let mut stmt = notes_db
        .prepare(
            "SELECT note_id, MAX(datetime(date)) AS last_edit
    FROM user_activity
    WHERE note_id IS NOT NULL
    GROUP BY note_id
    ORDER BY last_edit DESC
    LIMIT 3;
    ",
        )
        .context("Failed to prepare sql query")?;

    let mut handle = stmt.query([]).inspect_err(|err| tracing::error!(task="getting dashboard stats", status="error", error=?err, %user_uuid, "Failed to get 3 latest user edits")).context("Sqlite errro while getting lates 3 edits")?;

    while let Some(row) = handle.next().context("failed to get next row")? {
        let note_id: String = row.get(0).inspect_err(|err| tracing::error!(task="getting dashboard stats", error=?err, %user_uuid, "Failed to get note_id")).context("Db error while getting dashboard data")?;
        let datetime: String =row.get(1).inspect_err(|err| tracing::error!(task="getting dashboard stats", error=?err, %user_uuid, "Failed to get date")).context("Db error while getting dashboard data")?;
        last_three_edited.push((note_id, datetime));
    }
    let mut favourite_tags: Vec<(String, String)> = Vec::new();

    let mut stmt = notes_db
        .prepare(
            " SELECT
t.name,
t.color,
COUNT(nt.note_local_id) AS usage_count
FROM tags t
JOIN note_tags nt ON nt.tag_id = t.tag_id
JOIN notes n ON n.local_id = nt.note_local_id AND n.is_deleted = 0
GROUP BY t.tag_id, t.name, t.color
ORDER BY usage_count DESC
LIMIT 3;
",
        )
        .context("Failed to prepare sql query")?;

    let mut handle = stmt.query([]).inspect_err(|err| tracing::error!(task="getting dashboard stats", status="error", error=?err, %user_uuid, "Failed to get 3 favourite user tags")).context("Sqlite errro while getting 3 favourite tags ")?;

    while let Some(row) = handle.next().context("failed to get next row")? {
        let tag_name: String = row.get(0).inspect_err(|err| tracing::error!(task="getting dashboard stats", error=?err, %user_uuid, "Failed to get note_id")).context("Db error while getting dashboard data")?;
        let tag_colour: String = row.get(1).inspect_err(|err| tracing::error!(task="getting dashboard stats", error=?err, %user_uuid, "Failed to get tag colour")).context("Db error while getting dashboard data")?;

        favourite_tags.push((tag_name, tag_colour));
    }

    return Ok(DashboardData {
        number_of_notes,
        number_of_encrypted_notes,
        account_creation,
        activity_vec,
        last_three_edited,
        favourite_tags,
    });
}
