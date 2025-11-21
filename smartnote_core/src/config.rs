//! This module is responsible for configuring paths for applications do it by calling ProgramFiles::init()

use dirs_next::data_local_dir;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
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

pub struct CommonMetaInformation {
    device_id: uuid::Uuid,
    current_user: uuid::Uuid,
}

impl CommonMetaInformation {
    //call it after paths init and getting ids and current user to after logging in
    fn init(paths: &ProgramFiles) -> Result<CommonMetaInformation, crate::errors::Error> {
        Ok(CommonMetaInformation {
            device_id: get_device_id(&paths)?,

            current_user: read_current_user(paths.active_user_path.clone())?, //login will return current user
        })
    }
}

#[derive(Serialize, Deserialize)]

/// ConfigData contains states of aplication
pub struct ConfigData {
    pub fallback: bool,
    pub data_dir: PathBuf,
}

///determining fallback and creating paths and ProgramFiles struct
impl ProgramFiles {
    pub fn init() -> Result<ProgramFiles, crate::errors::Error> {
        let mut fallback = false;
        let base = data_local_dir();

        match base {
            Some(program_home_path) => {
                let active_user_path = program_home_path.join("smartnote/active_user.json");
                let user_uuid = match read_current_user(active_user_path) {
                    //temp uuid on first run
                    Ok(uuid) => uuid,
                    Err(_) => {
                        crate::services::logger::log_success(
                            "No active user found (first run?), using temp UUID",
                        );
                        uuid::Uuid::new_v4()
                    }
                };

                let program_paths = get_paths(
                    program_home_path.clone(),
                    user_uuid, //tu zmiana
                )?; //function users uuid also, its to add
                write_config(fallback, &program_paths)?;
                Ok(program_paths)
            }
            None => {
                fallback = true;
                let log_content = "Couldnt get directory, trying to fallback";
                crate::services::logger::log_error(log_content, "");
                let first_fallback = std::env::temp_dir(); //cleaner logic needed when it will be added
                let program_paths = get_paths(first_fallback, uuid::Uuid::new_v4())?;
                write_config(fallback, &program_paths)?;
                Ok(program_paths)
            }
        }
    }
}
///function which creates paths and create them in sense of getting current user
fn get_paths(
    program_home_path: PathBuf,
    user_uuid: uuid::Uuid,
) -> Result<ProgramFiles, crate::errors::Error> {
    let app_string = format!("smartnote/users/{}/", user_uuid); //in the future add uuid got from login
    let mut user_home_path = program_home_path.clone();
    user_home_path.push(app_string);
    std::fs::create_dir_all(&user_home_path)?;

    for path in ["notes", "assets", "logs", "tmp", "tmp_delete"] {
        //TODO check if keys should be stored in files
        let path_to_create = user_home_path.join(path);

        let log_content = format!("file paths created, app directory: {}", path);
        crate::services::logger::log_success(&log_content);
        std::fs::create_dir_all(path_to_create)?;
    }
    crate::services::logger::log_success("created subdirectories successfully");
    Ok(ProgramFiles {
        base: user_home_path.clone(),
        data_base_path: user_home_path.join("note.sqlite"),
        notes_path: user_home_path.join("notes"),
        assets_path: user_home_path.join("assets"),
        logs_path: user_home_path.join("logs/app.log"),
        config_path: user_home_path.join("config.json"),
        tmp_path: user_home_path.join("tmp"),
        delete_tmp_path: user_home_path.join("tmp_delete"),
        local_login_database_path: program_home_path.join("smartnote/users/local_login_db.sqlite"),
        device_id_path: program_home_path.join("smartnote/device_id.json"),
        active_user_path: program_home_path.join("smartnote/active_user.json"),
    })
}

///function responsible for writeing config data, current directory and if is it fallback
fn write_config(fallback: bool, program_paths: &ProgramFiles) -> Result<(), crate::errors::Error> {
    let config_content = ConfigData {
        fallback: fallback,
        data_dir: program_paths.base.to_path_buf(),
    };
    let content = serde_json::to_string_pretty(&config_content).inspect_err(|err| {
        crate::services::logger::log_error(
            "serializing error config will be empty string, error",
            err,
        )
    })?; //pretty
    crate::services::logger::log_success("serialized config content");

    fs::write(&program_paths.config_path, &content).inspect_err(|err| crate::services::logger::log_error(
            "couldnt write to config.json, it will try again to write only braccets or if it fail it will have no content or not existing",
            err,
        ))?;

    crate::services::logger::log_success("written content to config.json");

    Ok(())
}

/// function for getting device id, or creating new if not exists
pub fn get_device_id(paths: &ProgramFiles) -> Result<uuid::Uuid, crate::errors::Error> {
    if paths.device_id_path.exists() {
        let file_content = std::fs::read_to_string(&paths.device_id_path)?;
        let parsed_file: serde_json::Value = serde_json::from_str(&file_content)?;
        let device_id = uuid::Uuid::parse_str(
            parsed_file["device_id"]
                .as_str()
                .ok_or(crate::errors::Error::DeviceIdErorr)?,
        )?;
        Ok(device_id)
    } else {
        let device_id = uuid::Uuid::new_v4();

        let file_content = serde_json::json!({
                "device_id": device_id,
        });
        let file_content = serde_json::to_string_pretty(&file_content)?;
        std::fs::write(&paths.device_id_path, file_content)?;
        Ok(device_id)
    }
}
/// function to change or set actie user after registering/login/changing account
pub fn change_active_user(
    user_uuid: uuid::Uuid,
    paths: &ProgramFiles,
) -> Result<(), crate::errors::Error> {
    let data = serde_json::json!({
        "user_uuid": user_uuid
    });
    let file_content = serde_json::to_string_pretty(&data)?;
    std::fs::write(&paths.active_user_path, file_content)?;
    crate::services::logger::log_success("changed current user");
    Ok(())
} //cos tutaj sie wypierdala

fn read_current_user(path: PathBuf) -> Result<uuid::Uuid, crate::errors::Error> {
    let file_content = std::fs::read_to_string(&path)?;
    let contents_json: serde_json::Value = serde_json::from_str(&file_content)?;
    let user_uuid = uuid::Uuid::parse_str(
        contents_json["user_uuid"]
            .as_str()
            .ok_or(crate::errors::Error::CurrentUserNotFound)?,
    )?;
    crate::services::logger::log_success("got current user uuid");
    Ok(user_uuid)
}
#[test]
fn init_test() {
    let paths = ProgramFiles::init().unwrap();
    println!("{:#?}", paths);
    assert!(paths.config_path.exists())
}

#[test]

fn test_changing_user() {
    let paths = ProgramFiles::init().unwrap();
    change_active_user(uuid::Uuid::new_v4(), &paths).unwrap();
}

#[test]
fn test_creating_device_id() {
    let paths = ProgramFiles::init().unwrap();
    let device_id = get_device_id(&paths).unwrap();
    println!("{}", device_id);
}
