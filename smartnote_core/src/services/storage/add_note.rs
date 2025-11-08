pub fn add_note(conn: &rusqlite::Connection) {
    let mut stmt = conn.prepare("BEGIN; INSERT INTO notes (local_id, mongo_id, owner_id, name, title, summary, content_path, created_at, updated_at, deleted_at, version, cloud_version, sync_state, is_deleted, ecrypted, crypto_meta) VALUES (:local_id, :mongo_id, :owner_id, :name, :title, :summary, :content_path, :created_at, :updated_at, :deleted_at, :version, :cloud_version, :sync_state, :is_deleted, :encrypted, :crypto_meta; COMMIT;  )")?;
    //this note
    let note = crate::models::note::Note {
        title: "test".to_string(),
    }; //fill test note

    //if couldnt insert note, add note struct content into json file with name to backup
    //dodać sprawdzanie na name czy nie istnieje takie albo na local_id
    //if anything fails add to this config fail, if prepare fail i think i will fail every time
}
