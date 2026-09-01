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

    if version < 2 {
        let tx = users_db
            .unchecked_transaction()
            .context("failed to create transaction")?;

        if !column_exists(&tx, "users_data", "kek_argon_params")? {
            tx.execute(
                "ALTER TABLE users_data ADD COLUMN kek_argon_params TEXT",
                [],
            )
            .context("failed to add users_data.kek_argon_params")?;

            tx.execute(r#"UPDATE users_data SET kek_argon_params = '{"m_cost": 19456, "t_cost": 2, "p_cost": 1}'"#, []).context("failed to update params")?;
        }

        tx.pragma_update(None, "user_version", 2)
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

    if version < 3 {
        let tx = users_db
            .unchecked_transaction()
            .context("failed to create transaction")?;

        if !column_exists(&tx, "users_data", "online_account_id")? {
            tx.execute(
                "ALTER TABLE users_data ADD COLUMN online_account_id TEXT",
                [],
            )
            .context("failed to add users_data.online_account_id")?;
        }

        tx.pragma_update(None, "user_version", 3)
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
    let version: i64 = notes_db
        .query_row("PRAGMA user_version;", [], |r| r.get(0))
        .context("failed to read notes db version")?;

    if version >= crate::constants::NOTES_DB_VERSION {
        return Ok(());
    }

    if version < 1 && column_exists(notes_db, "notes", "name")? {
        // `name` carries UNIQUE(owner_id, name), implemented as an automatic
        // index SQLite will not let ALTER TABLE ... DROP COLUMN remove.
        // Full table rebuild is the only way to drop it.
        notes_db
            .pragma_update(None, "foreign_keys", "OFF")
            .context("failed to disable foreign_keys for notes table rebuild")?;

        let tx = notes_db
            .unchecked_transaction()
            .context("failed to create migration transaction")?;

        tx.execute_batch(
            "
            CREATE TABLE notes_new (
                local_id TEXT PRIMARY KEY,
                mongo_id TEXT,
                owner_id TEXT NOT NULL,

                title TEXT NOT NULL,
                summary TEXT NOT NULL,
                content_path TEXT NOT NULL,

                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                deleted_at INTEGER,

                version INTEGER NOT NULL DEFAULT 1,
                cloud_version INTEGER DEFAULT NULL,

                sync_state TEXT NOT NULL DEFAULT 'LocalOnly',
                is_deleted INTEGER NOT NULL DEFAULT 0,

                encrypted INTEGER NOT NULL DEFAULT 1,
                crypto_meta TEXT,

                CHECK(sync_state IN ('LocalOnly', 'PendingUpload', 'Synced', 'Conflict', 'Error', 'PendingDeleted'))
            );

            INSERT INTO notes_new (
                local_id, mongo_id, owner_id, title, summary, content_path,
                created_at, updated_at, deleted_at, version, cloud_version,
                sync_state, is_deleted, encrypted, crypto_meta
            )
            SELECT
                local_id, mongo_id, owner_id,
                COALESCE(title, ''), COALESCE(summary, ''), COALESCE(content_path, ''),
                created_at, updated_at, deleted_at, version, cloud_version,
                sync_state, is_deleted, encrypted, crypto_meta
            FROM notes;

            DROP TABLE notes;
            ALTER TABLE notes_new RENAME TO notes;

            CREATE INDEX IF NOT EXISTS idx_notes_owner_updated ON notes(owner_id, updated_at DESC);
            CREATE INDEX IF NOT EXISTS idx_notes_sync_state ON notes(sync_state);
            CREATE INDEX IF NOT EXISTS idx_notes_mongo_id ON notes(mongo_id);
            ",
        )
        .context("failed to rebuild notes table without name column")?;

        tx.commit()
            .inspect_err(|e| {
                tracing::error!(
                    task = "notes database migration",
                    status = "error",
                    error = ?e,
                    version,
                    "failed to commit migration transaction"
                )
            })
            .context("failed to commit notes migration transaction")?;

        let violation_count: i64 = notes_db
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| {
                r.get(0)
            })
            .context("failed to run foreign_key_check after notes rebuild")?;
        if violation_count > 0 {
            return Err(crate::errors::Error::InternalError(
                "notes table rebuild left dangling foreign key references".into(),
            ));
        }

        notes_db
            .pragma_update(None, "foreign_keys", "ON")
            .context("failed to re-enable foreign_keys after notes table rebuild")?;
    }

    if version < 2 {
        notes_db
            .pragma_update(None, "foreign_keys", "OFF")
            .context("failed to disable foreign_keys for notes migration")?;

        let tx = notes_db
            .unchecked_transaction()
            .context("failed to create notes migration transaction")?;

        tx.execute_batch(
            r#"
        CREATE TABLE notes_new (
            local_id TEXT PRIMARY KEY,
            mongo_id TEXT,
            owner_id TEXT NOT NULL,

            title TEXT NOT NULL,
            summary TEXT NOT NULL,
            content_path TEXT NOT NULL,

            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            deleted_at INTEGER,

            version INTEGER NOT NULL DEFAULT 1,
            cloud_version INTEGER DEFAULT NULL,

            sync_state TEXT NOT NULL DEFAULT 'LocalOnly',
            is_deleted INTEGER NOT NULL DEFAULT 0,

            encrypted INTEGER NOT NULL DEFAULT 1,
            crypto_meta TEXT,

            CHECK(sync_state IN (
                'LocalOnly',
                'PendingUpload',
                'Synced',
                'Conflict',
                'Error',
                'PendingDeleted',
                'WaitingForTombstone'
            ))
        );

        INSERT INTO notes_new (
            local_id,
            mongo_id,
            owner_id,
            title,
            summary,
            content_path,
            created_at,
            updated_at,
            deleted_at,
            version,
            cloud_version,
            sync_state,
            is_deleted,
            encrypted,
            crypto_meta
        )
        SELECT
            local_id,
            mongo_id,
            owner_id,
            title,
            summary,
            content_path,
            created_at,
            updated_at,
            deleted_at,
            version,
            cloud_version,
            sync_state,
            is_deleted,
            encrypted,
            crypto_meta
        FROM notes;

        DROP TABLE notes;

        ALTER TABLE notes_new RENAME TO notes;

        CREATE INDEX IF NOT EXISTS idx_notes_owner_updated
            ON notes(owner_id, updated_at DESC);

        CREATE INDEX IF NOT EXISTS idx_notes_sync_state
            ON notes(sync_state);

        CREATE INDEX IF NOT EXISTS idx_notes_mongo_id
            ON notes(mongo_id);
        "#,
        )
        .context("failed to migrate notes table to version 2")?;

        tx.commit()
            .inspect_err(|e| {
                tracing::error!(
                    task = "notes database migration",
                    status = "error",
                    error = ?e,
                    version,
                    "failed to commit version 2 migration"
                )
            })
            .context("failed to commit notes version 2 migration")?;

        notes_db
            .pragma_update(None, "foreign_keys", "ON")
            .context("failed to re-enable foreign_keys after notes migration")?;
    }

    notes_db
        .pragma_update(None, "user_version", crate::constants::NOTES_DB_VERSION)
        .context("failed to update notes db version")?;

    Ok(())
}
