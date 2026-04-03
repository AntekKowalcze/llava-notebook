use anyhow::Context;

fn column_exists(
    conn: &rusqlite::Connection,
    table: &str,
    column: &str,
) -> Result<bool, crate::errors::Error> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .context("failed to prepare pragma table_info")?;
    let mut rows = stmt
        .query([])
        .context("failed to query pragma table_info")?;

    while let Some(row) = rows
        .next()
        .context("failed to read pragma table_info row")?
    {
        let current: String = row
            .get(1)
            .context("failed to get column name from pragma table_info")?;
        if current == column {
            return Ok(true);
        }
    }

    Ok(false)
}

pub fn run_users_migration(users_db: &rusqlite::Connection) -> Result<(), crate::errors::Error> {
    let version = users_db
        .query_row("PRAGMA user_version;", [], |r| r.get(0))
        .unwrap_or(0);

    if version < 1 {
        let tx = users_db
            .unchecked_transaction()
            .context("failed to create transaction")?;

        if !column_exists(&tx, "users_data", "master_key_enc")? {
            tx.execute("ALTER TABLE users_data ADD COLUMN master_key_enc BLOB", [])
                .context("failed to add users_data.master_key_enc")?;
        }
        if !column_exists(&tx, "users_data", "master_key_nonce")? {
            tx.execute(
                "ALTER TABLE users_data ADD COLUMN master_key_nonce BLOB",
                [],
            )
            .context("failed to add users_data.master_key_nonce")?;
        }
        if !column_exists(&tx, "users_data", "master_kek_salt")? {
            tx.execute("ALTER TABLE users_data ADD COLUMN master_kek_salt TEXT", [])
                .context("failed to add users_data.master_kek_salt")?;
        }

        tx.pragma_update(None, "user_version", crate::constants::USERS_DB_VERSION)
            .context("Failed to update db version")?;
        tx.commit()
            .inspect_err(|e| {
                tracing::error!(
                    task = "migrating database",
                    status = "error",
                    error = ?e,
                    %version,
                    "failed to commit transaction"
                )
            })
            .context("Failed to commit migration transaction on users database")?;
    }

    Ok(())
}

pub fn run_notes_migration(notes_db: &rusqlite::Connection) -> Result<(), crate::errors::Error> {
    let version = notes_db
        .query_row("PRAGMA user_version;", [], |r| r.get(0))
        .unwrap_or(0);

    if version < crate::constants::NOTES_DB_VERSION {
        let tx = notes_db
            .unchecked_transaction()
            .context("failed to create transaction")?;
        // Notes DB has no step migrations yet; keep this path for future versions.
        tx.pragma_update(None, "user_version", crate::constants::NOTES_DB_VERSION)
            .context("failed to update notes db version")?;
        tx.commit()
            .context("failed to commit notes migration transaction")?;
    }

    Ok(())
}
