use llava_core::{models::note, *};
use rusqlite::named_params;
use rusqlite::*;
use zeroize::{Zeroize, Zeroizing};
#[test]
fn program_flow() {
    let _ = clear_tmp();
    let note_name = "test_note".to_string();
    let program_paths =
        llava_core::ProgramFiles::init_in_base().expect("failed creating program pahts");

    let _logger_worker = llava_core::configure_logger(&program_paths.logs_path)
        .expect("failed creating logger guard");
    let mut local_login_db_conn =
        llava_core::connect_or_create_local_login_db(&program_paths.local_login_database_path)
            .expect("failed creating local user db");

    llava_core::register_user_offline(
        "test".to_string(),
        Zeroizing::from("ZAQ!2wsx".to_string()),
        Zeroizing::from("ZAQ!2wsx".to_string()),
        &program_paths,
        &mut local_login_db_conn,
    )
    .unwrap();

    // Assert: user in local database
    let user_exists: bool = local_login_db_conn
        .query_row(
            "SELECT 1 FROM users_data WHERE username = :name",
            named_params! {":name": "test"},
            |_| Ok(true),
        )
        .optional()
        .unwrap()
        .unwrap_or(false);
    assert!(user_exists, "User should exist in local DB");

    let (mut current_user, some, some_two) = llava_core::local_log_in(
        "test".to_string(),
        Zeroizing::from("ZAQ!2wsx".to_string()),
        &mut local_login_db_conn,
        &program_paths,
    )
    .expect("failed to get current user");
    let mut state = AppState::init().expect("failed creating app state");

    state.current_user = std::sync::Mutex::new(Some(current_user));
    let mut note_db_conn =
        llava_core::get_connection(&program_paths).expect("failed creating notes db");

    let owner_id = state
        .current_user
        .lock()
        .expect("failed to lock")
        .ok_or(crate::errors::Error::CurrentUserNotFound)
        .expect("failed to read owner_id");

    llava_core::add_note_to_database(
        &mut note_db_conn,
        &program_paths,
        note_name.clone(),
        owner_id,
    )
    .expect("failed adding notes to db");

    let note_file_path = program_paths.notes_path.join(format!("{}.md", note_name));
    assert!(note_file_path.exists(), "Note file should exist");

    let count: i64 = note_db_conn
        .query_row(
            "SELECT COUNT(*) FROM notes WHERE name = :name AND owner_id = :owner",
            named_params! {":name": note_name, ":owner": owner_id.to_string()},
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1, "Should have exactly 1 note in DB");

    let note_id: String = note_db_conn
        .query_row(
            "SELECT local_id FROM notes WHERE name = :note_name",
            named_params! {
                ":note_name": note_name.clone()
            },
            |row| row.get("local_id"),
        )
        .expect("failed to get id of note");
    let note_id = uuid::Uuid::parse_str(&note_id).expect("failed to parse uuid");
    llava_core::update_md(
        &note_db_conn,
        note_name.clone(),
        note_id,
        "this is content from integrated test".to_string(),
        &program_paths,
        "integration test".to_string(),
    )
    .expect("failed to update note");

    // Assert: content faktycznie się zmienił
    let updated_content = std::fs::read_to_string(&note_file_path).unwrap();
    assert!(updated_content.contains("this is content from integrated test"));

    // Assert: title i summary w bazie
    let (title, summary): (String, String) = note_db_conn
        .query_row(
            "SELECT title, summary FROM notes WHERE local_id = :id",
            named_params! {":id": note_id.to_string()},
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(title, "integration test");
    assert!(!summary.is_empty());

    let note_content = llava_core::read_note_content(&program_paths, note_name.clone())
        .expect("failed to read content from file");

    llava_core::delete_note(
        &mut note_db_conn,
        note_name.clone(),
        note_id,
        &program_paths,
    )
    .expect("failed to delete note");
    // Assert: plik przeniesiony do delete_tmp
    let deleted_file_path = program_paths
        .delete_tmp_path
        .join(format!("{}.md", note_name));
    assert!(deleted_file_path.exists(), "Note should be in delete_tmp");
    assert!(!note_file_path.exists(), "Note should not be in notes dir");

    // Assert: soft delete w bazie
    let (is_deleted, deleted_at, sync_state): (bool, Option<i64>, String) = note_db_conn
        .query_row(
            "SELECT is_deleted, deleted_at, sync_state FROM notes WHERE local_id = :id",
            named_params! {":id": note_id.to_string()},
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert!(
        !is_deleted,
        "is_deleted should be false, after 30 days should be true"
    );
    assert!(deleted_at.is_some(), "deleted_at should be set");
    assert_eq!(sync_state, "PendingDeleted");
}

fn clear_tmp() -> Result<(), std::io::Error> {
    let path = std::env::temp_dir().join("llava_test");
    std::fs::remove_dir_all(path)?;
    Ok(())
}
