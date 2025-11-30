//! This module is responsible for configuring paths for applications do it by calling ProgramFiles::init()
use crate::constants::*;
use crate::utils::{Format, log_helper};
use anyhow::Context;
use dirs_next::data_local_dir;
use rusqlite::{Connection, named_params};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::{
    fs::{self, create_dir_all},
    path::PathBuf,
    sync::Mutex,
};
#[derive(Debug)]
/// Struct with important paths
pub struct ProgramFiles {
    pub base: PathBuf,
    pub data_base_path: PathBuf,
    pub notes_path: PathBuf,
    pub assets_path: PathBuf,
    pub logs_path: PathBuf,
    pub config_path: PathBuf,
    pub tmp_path: PathBuf,
    pub delete_tmp_path: PathBuf,
    pub local_login_database_path: PathBuf,
    pub device_id_path: PathBuf,
    pub active_user_path: PathBuf,
}

pub struct AppState {
    pub device_id: uuid::Uuid,
    pub current_user: Mutex<Option<uuid::Uuid>>,
    pub connection: Mutex<Option<Connection>>,
    pub username: Mutex<Option<String>>,
    pub paths: Mutex<Option<ProgramFiles>>,
}

impl AppState {
    pub fn init() -> Result<AppState, crate::errors::Error> {
        Ok(AppState {
            device_id: get_device_id()?,
            current_user: Mutex::new(None),
            connection: Mutex::new(None),
            username: Mutex::new(None),
            paths: Mutex::new(None), //login will return current user
        })
    }
    //mayby add updating currentuser to change current user, and updating connection go get connection and only get username here, or even in register
}

#[derive(Serialize, Deserialize)]

/// ConfigData contains states of aplication
pub struct ConfigData {
    pub data_dir: PathBuf,
}

///determining fallback and creating paths and ProgramFiles struct
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

                uuid::Uuid::new_v4()
            }
        };

        let program_paths = get_paths(
            program_home_path.clone(),
            user_uuid, //tu zmiana
        )?; //function users uuid also, its to add
        write_config(&program_paths)?;
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

                uuid::Uuid::new_v4()
            }
        };

        let program_paths = get_paths(program_home_path.clone(), user_uuid)?; //function users uuid also, its to add
        write_config(&program_paths)?;
        Ok(program_paths)
    }
}

///function which creates paths and create them in sense of getting current user
fn get_paths(
    program_home_path: PathBuf,
    user_uuid: uuid::Uuid,
) -> Result<ProgramFiles, crate::errors::Error> {
    let app_string = format!("{}/{}/", USER_DIR_PATTERN, user_uuid); //in the future add uuid got from login
    let mut user_home_path = program_home_path.clone();
    user_home_path.push(app_string);
    std::fs::create_dir_all(&user_home_path)?;

    for path in SUBDIRS {
        //TODO check if keys should be stored in files
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
        logs_path: user_home_path.join(LOGS_PATH),
        config_path: user_home_path.join(CONFIG_FILE),
        tmp_path: user_home_path.join("tmp"),
        delete_tmp_path: user_home_path.join("tmp_delete"),
        local_login_database_path: program_home_path.join(LOCAL_USERS_DB),
        device_id_path: program_home_path.join(DEVICE_ID_FILE),
        active_user_path: program_home_path.join(ACTIVE_USER_JSON_PATH),
    })
}

///function responsible for writeing config data, current directory and if is it fallback
fn write_config(program_paths: &ProgramFiles) -> Result<(), crate::errors::Error> {
    let config_content = ConfigData {
        data_dir: program_paths.base.to_path_buf(),
    };
    let content = serde_json::to_string_pretty(&config_content)
        .inspect_err(|err| {
            crate::services::logger::log_error(
                "serializing error config will be empty string, error",
                err,
            )
        })
        .context("couldnt parse config content into json")?; //pretty
    crate::services::logger::log_success("serialized config content");

    fs::write(&program_paths.config_path, &content).inspect_err(|err| {
        tracing::error!(
            task = "writing config to json",
            status = "error",
            "couldnt write config to json"
        );
    })?;

    log_helper(
        "writing config to json",
        "success",
        None::<Format<String>>,
        "Written config to json",
    );
    Ok(())
}

/// function for getting device id, or creating new if not exists
pub fn get_device_id() -> Result<uuid::Uuid, crate::errors::Error> {
    let home_path = data_local_dir().ok_or(crate::errors::Error::FatalError)?;
    let mut device_id_path = home_path.join("llava");
    create_dir_all(&device_id_path)?;
    device_id_path = home_path.join("llava/device_id.json");

    if device_id_path.exists() {
        let file_content = std::fs::read_to_string(&device_id_path)?;
        let parsed_file: serde_json::Value = serde_json::from_str(&file_content)
            .context("couldnt parse device id file content with serde")?;
        let device_id = uuid::Uuid::parse_str(
            parsed_file[DEVICE_ID_JSON_KEY]
                .as_str()
                .ok_or(crate::errors::Error::DeviceIdErorr)
                .context("device id not found in device id file")?,
        )
        .context("couldnt get device id from device id file ")?;
        Ok(device_id)
    } else {
        let device_id = uuid::Uuid::new_v4();

        let file_content = serde_json::json!({
                DEVICE_ID_JSON_KEY: device_id,
        });
        let file_content = serde_json::to_string_pretty(&file_content)
            .context("couldnt parse device uuid to json ")?;
        std::fs::write(&device_id_path, file_content)?;
        Ok(device_id)
    }
}
/// function to change or set actie user after registering/login/changing account
pub fn change_active_user(
    user_uuid: uuid::Uuid,
    paths: &ProgramFiles,
) -> Result<(), crate::errors::Error> {
    let data = serde_json::json!({
        ACTIVE_USER_JSON_KEY: user_uuid
    });
    let file_content = serde_json::to_string_pretty(&data)
        .context("failed to parse user uuid to json when changin active user")?;
    std::fs::write(&paths.active_user_path, file_content)?;
    crate::services::logger::log_success("changed current user");
    Ok(())
}

fn read_current_user(path: &PathBuf) -> Result<uuid::Uuid, crate::errors::Error> {
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
    assert!(paths.config_path.exists())
}

#[test]

fn test_changing_user() {
    let paths = ProgramFiles::init_in_base().unwrap();
    change_active_user(uuid::Uuid::new_v4(), &paths).unwrap();
}

#[test]
fn test_creating_device_id() {
    let paths = ProgramFiles::init_in_base().unwrap();

    let device_id = get_device_id().unwrap();
    println!("{}", device_id);
}
