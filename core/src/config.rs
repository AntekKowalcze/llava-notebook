//! # Application path configuration module
//! **Purpose**: This module is responsible for resolving, creating, and persisting all filesystem
//! paths the application relies on. Call [`ProgramFiles::init`] at startup to get a fully
//! initialised [`ProgramFiles`] struct. All user data is isolated under a per-UUID subdirectory,
//! so switching accounts only requires calling [`get_paths`] with a new UUID.
//!
//! ## Exported items
//! * [`ProgramFiles`] — Struct holding every important path (databases, notes, assets, logs, config,
//!   tmp dirs, device-id file, active-user file). Passed by reference throughout the application
//! * [`AppState`] — Tauri application state; wraps all shared runtime values (`device_id`,
//!   `current_user`, both DB connections, `username`, `paths`) in `Mutex<Option<T>>` for
//!   thread-safe lazy initialisation
//! * [`ConfigData`] — Serialisable struct written to `config.json`; currently stores `data_dir`
//! * [`ProgramFiles::init`] — Production initialiser; resolves base via `dirs_next::data_local_dir`,
//!   reads active user UUID from `active_user.json` (falls back to nil UUID on first run),
//!   creates all subdirectories, and writes `config.json`
//! * [`ProgramFiles::init_in_base`] — Test initialiser; uses `std::env::temp_dir()/llava_test`
//!   as the base instead of the system data directory
//! * [`get_paths`] — Builds a [`ProgramFiles`] for a given `(home, uuid)` pair, creates all
//!   required subdirectories on disk (via `SUBDIRS` constant), and returns the struct
//! * [`get_device_id`] — Reads the device UUID from `device_id.json`; if the file is missing
//!   a new UUIDv4 is generated and written; if the file is corrupt the UUID is recovered from
//!   the `users_data` table and the file is rewritten
//! * [`change_active_user`] — Writes a new UUID into `active_user.json`; called after
//!   login, registration, or account switching
//! * [`read_current_user`] — Reads and parses the UUID from `active_user.json`
//!
//! ## Key design decisions
//! On first run no `active_user.json` exists, so [`read_current_user`] will fail;
//! `init` / `init_in_base` catch this error and substitute `Uuid::nil()` as a temporary
//! placeholder. The nil-UUID directory is cleaned up by `after_validation` in the auth module
//! once a real user logs in or registers. All user data lives under
//! `{app_home}/{USER_DIR_PATTERN}/{uuid}/`, which means account isolation is purely filesystem-level.
//! `AppState` stores everything behind `Mutex<Option<T>>` so Tauri commands can mutate shared
//! state safely without async locks.
//!
//! ## Dependencies
//! - `dirs_next` — Resolves the platform data-local directory (`%APPDATA%`, `~/.local/share`, etc.)
//! - `serde` / `serde_json` — Serialisation of [`ConfigData`] and device-id / active-user JSON files
//! - `rusqlite` — DB fallback in [`get_device_id`] when the device-id file is unreadable
//! - `uuid` — UUIDv4 generation for new device IDs and nil-UUID sentinel on first run
//! - `std::sync::Mutex` — Thread-safe shared state in [`AppState`]
use crate::constants::*;
use crate::utils::{Format, log_helper};
use anyhow::Context;
use dirs_next::data_local_dir;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use std::{path::PathBuf, sync::Mutex};
#[derive(Debug, Clone)]
pub struct ProgramFiles {
    pub base: PathBuf,
    pub data_base_path: PathBuf,
    pub notes_path: PathBuf,
    pub assets_path: PathBuf,
    pub logs_path: PathBuf,
    pub config_path: PathBuf,
    pub config_backup_path: PathBuf,
    pub tmp_path: PathBuf,
    pub delete_tmp_path: PathBuf,
    pub local_login_database_path: PathBuf,
    pub device_id_path: PathBuf,
    pub active_user_path: PathBuf,
    pub app_home: PathBuf,
}
#[derive(Default, Debug)]
pub struct AppState {
    pub device_id: Mutex<Option<uuid::Uuid>>,
    pub current_user: Mutex<Option<uuid::Uuid>>,
    pub users_db: Mutex<Option<Connection>>,
    pub notes_db: Mutex<Option<Connection>>,

    pub username: Mutex<Option<String>>,
    pub paths: Mutex<Option<ProgramFiles>>,
    pub user_config: Mutex<Option<std::collections::HashMap<String, String>>>,
}

impl AppState {
    pub fn init() -> Result<AppState, crate::errors::Error> {
        Ok(AppState {
            device_id: Mutex::new(None),
            current_user: Mutex::new(None),
            users_db: Mutex::new(None), //now for current user db add then for current note  db
            notes_db: Mutex::new(None),
            username: Mutex::new(None),
            paths: Mutex::new(None), //login will return current user
            user_config: Mutex::new(None),
        })
    }
}
#[derive(Serialize, Deserialize)]

pub struct ConfigData {
    pub data_dir: PathBuf,
}
impl ProgramFiles {
    pub fn init() -> Result<ProgramFiles, crate::errors::Error> {
        let program_home_path = data_local_dir()
            .ok_or(crate::errors::Error::FatalError)
            .inspect_err(|_| {
                tracing::error!(
                    task = "initializating paths",
                    status = "error",
                    "Fatal error, couldnt get main path for program",
                )
            })?;

        let active_user_path = program_home_path.join(ACTIVE_USER_JSON_PATH);
        let user_uuid = match read_current_user(&active_user_path) {
            //temp uuid on first run
            Ok(uuid) => uuid,
            Err(err) => {
                tracing::error!(
                    task = "initializating note",
                    status = "error",
                    ?err,
                    "Cannot get user uuid, possibly first run"
                );
                uuid::Uuid::nil()
            }
        };

        let program_paths = get_paths(program_home_path.clone(), &user_uuid)?;
        // write_config(&program_paths)?;
        Ok(program_paths)
    }

    pub fn init_in_base() -> Result<ProgramFiles, crate::errors::Error> {
        let program_home_path: PathBuf = std::env::temp_dir().join("llava_test");
        let active_user_path = program_home_path.join(ACTIVE_USER_JSON_PATH);
        let user_uuid = match read_current_user(&active_user_path) {
            //temp uuid on first run
            Ok(uuid) => uuid,
            Err(err) => {
                tracing::error!(
                    task = "paths config note",
                    status = "error",
                    ?err,
                    "Cannot get user uuid, possibly first run"
                );

                uuid::Uuid::nil()
            }
        };

        let program_paths = get_paths(program_home_path.clone(), &user_uuid)?; //function users uuid also, its to add
        // write_config(&program_paths)?;
        Ok(program_paths)
    }
}
pub fn get_paths(
    program_home_path: PathBuf,
    user_uuid: &uuid::Uuid,
) -> Result<ProgramFiles, crate::errors::Error> {
    let app_string = format!("{}/{}/", USER_DIR_PATTERN, user_uuid);
    let mut user_home_path = program_home_path.clone();
    user_home_path.push(app_string);
    std::fs::create_dir_all(&user_home_path)?;

    for path in SUBDIRS {
        let path_to_create = user_home_path.join(path);

        log_helper(
            "gettign paths",
            "success",
            Some(Format::Debug(path)),
            "file paths created",
        );
        std::fs::create_dir_all(path_to_create)?;
    }

    log_helper(
        "gettign paths",
        "success",
        None::<Format<String>>,
        "Created subdirs successfully",
    );

    Ok(ProgramFiles {
        base: user_home_path.clone(),
        data_base_path: user_home_path.join(NOTES_DB),
        notes_path: user_home_path.join("notes"),
        assets_path: user_home_path.join("assets"),
        logs_path: program_home_path.join(LOGS_PATH),
        config_path: user_home_path.join(CONFIG_FILE),
        config_backup_path: user_home_path.join(CONFIG_BACKUP_FILE),
        tmp_path: user_home_path.join("tmp"),
        delete_tmp_path: user_home_path.join("tmp_delete"),
        local_login_database_path: program_home_path.join(LOCAL_USERS_DB),
        device_id_path: program_home_path.join(DEVICE_ID_FILE),
        active_user_path: program_home_path.join(ACTIVE_USER_JSON_PATH),
        app_home: program_home_path.clone(),
    })
}

pub fn get_device_id(
    local_conn: &Connection,
    device_id_path: &PathBuf,
) -> Result<uuid::Uuid, crate::errors::Error> {
    // conn for local use db
    if let Some(parent) = device_id_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if device_id_path.exists() {
        let has_error: Result<uuid::Uuid, crate::errors::Error> = {
            let file_content = std::fs::read_to_string(&device_id_path)?;
            let parsed_file: serde_json::Value = serde_json::from_str(&file_content)
                .context("couldnt parse device id file content with serde")?;

            let device_id = uuid::Uuid::parse_str(
                parsed_file[DEVICE_ID_JSON_KEY]
                    .as_str()
                    .ok_or(crate::errors::Error::DeviceIdError)
                    .context("device id not found in device id file")?,
            )
            .context("failed to parse uuid")?;
            Ok(device_id)
        };
        match has_error {
            //if exists and couldnt read it just find it in db and write again if failes we have problems
            Ok(device_id_ok) => Ok(device_id_ok),
            Err(_err) => {
                let db_id: Result<String, rusqlite::Error> =
                    local_conn.query_row("SELECT device_id FROM users_data LIMIT 1", (), |row| {
                        row.get(0)
                    });
                if let Ok(id) = db_id {
                    let dev_uuid = uuid::Uuid::parse_str(&id).context("failed to parse uuid")?;
                    let json_data = serde_json::json!({
                        (DEVICE_ID_JSON_KEY): dev_uuid
                    });

                    // Convert the OBJECT to string, not the bare UUID
                    let file_content = serde_json::to_string_pretty(&json_data)
                        .context("couldnt serialize device id json")?;

                    std::fs::write(&device_id_path, file_content)
                        .context("couldnt write device id content")?;
                    Ok(dev_uuid)
                } else {
                    return Err(crate::errors::Error::FatalError);
                }
            }
        }
    } else {
        let device_id = uuid::Uuid::new_v4();
        let file_content = serde_json::json!({
                DEVICE_ID_JSON_KEY: device_id,
        });
        let file_content = serde_json::to_string_pretty(&file_content)
            .context("couldnt parse device uuid to json ")?;
        std::fs::write(&device_id_path, file_content).context("couldnt write device id content")?;
        Ok(device_id)
    }
}
pub fn change_active_user(
    user_uuid: &uuid::Uuid,
    paths: &ProgramFiles,
) -> Result<(), crate::errors::Error> {
    let data = serde_json::json!({
        ACTIVE_USER_JSON_KEY: &user_uuid
    });
    let file_content = serde_json::to_string_pretty(&data)
        .context("failed to parse user uuid to json when changin active user")?;
    std::fs::write(&paths.active_user_path, file_content)?;
    crate::services::logger::log_success("changed current user");
    Ok(())
}

pub fn read_current_user(path: &PathBuf) -> Result<uuid::Uuid, crate::errors::Error> {
    let file_content = std::fs::read_to_string(&path)?;
    let contents_json: serde_json::Value =
        serde_json::from_str(&file_content).context("failed to parse active_user.json file")?;
    let user_uuid = uuid::Uuid::parse_str(
        contents_json[ACTIVE_USER_JSON_KEY]
            .as_str()
            .ok_or(crate::errors::Error::CurrentUserNotFound)
            .context("There was no current user written in active_user.json file")?,
    )
    .context("couldnt get current user from active_user.json file")?;
    log_helper(
        "read user uuid",
        "success",
        Some(Format::Debug(&user_uuid)),
        "Successfully got user uuid",
    );
    Ok(user_uuid)
}
#[test]
fn init_test() {
    let paths = ProgramFiles::init_in_base().unwrap();
    println!("{:#?}", paths);
    assert!(paths.base.exists())
}

#[test]

fn test_changing_user() {
    let paths = ProgramFiles::init_in_base().unwrap();
    change_active_user(&uuid::Uuid::new_v4(), &paths).unwrap();
}

#[test]
fn test_creating_device_id() {
    let paths = ProgramFiles::init_in_base().unwrap();
    let local_conn = crate::services::auth::database_creation::connect_or_create_local_login_db(
        &paths.local_login_database_path,
    )
    .unwrap();
    let device_id = get_device_id(&local_conn, &paths.device_id_path).unwrap();
    println!("{}", device_id);
}
