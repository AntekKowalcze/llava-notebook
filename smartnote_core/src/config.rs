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
    pub keys_path: PathBuf,
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
    fn init(device_id: uuid::Uuid, current_user: uuid::Uuid) -> CommonMetaInformation {
        CommonMetaInformation {
            device_id: device_id,
            current_user: current_user, //login will return current user
        }
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
                let program_paths = get_paths(
                    program_home_path.clone(),
                    read_current_user(program_home_path.join("smartnote/active_user.json"))?,
                ); //function users uuid also, its to add
                write_config(fallback, &program_paths);
                Ok(program_paths)
            }
            None => {
                fallback = true;
                let log_content = "Couldnt get directory, trying to fallback";
                crate::services::logger::log_error(log_content, "");
                let first_fallback = std::env::temp_dir(); //cleaner logic needed when it will be added
                let program_paths = get_paths(first_fallback, uuid::Uuid::new_v4());
                write_config(fallback, &program_paths);
                Ok(program_paths)
            }
        }
    }
}
///function which creates paths and create them in sense of getting current user
fn get_paths(program_home_path: PathBuf, user_uuid: uuid::Uuid) -> ProgramFiles {
    let app_string = format!("smartnote/users/{}/", user_uuid); //in the future add uuid got from login
    let mut user_home_path = program_home_path.clone();
    user_home_path.push(app_string);
    if let Err(err) = std::fs::create_dir_all(&user_home_path) {
        if err.kind() == std::io::ErrorKind::PermissionDenied {
            crate::services::logger::log_error(
                "permission denied for creating program directories",
                &err,
            );
        } else if err.kind() == std::io::ErrorKind::AlreadyExists {
            crate::services::logger::log_success("Program directory already exists");
        } else if err.kind() == std::io::ErrorKind::NotFound {
            crate::services::logger::log_error("Parent directory not found", &err);
        } else {
            crate::services::logger::log_error("couldnt create program directory", &err);
        }
    } else {
        for path in ["notes", "assets", "keys", "logs", "tmp", "tmp_delete"] {
            //TODO check if keys should be stored in files
            let path_to_create = user_home_path.join(path);
            println!("{:?}", path_to_create);
            match std::fs::create_dir_all(path_to_create) {
                Ok(_) => {
                    crate::services::logger::log_success("created subdirectories successfully");
                }
                Err(err) => {
                    if err.kind() == std::io::ErrorKind::PermissionDenied {
                        crate::services::logger::log_error(
                            "permission denied for creating subdirectories",
                            &err,
                        );
                    } else if err.kind() == std::io::ErrorKind::AlreadyExists {
                        crate::services::logger::log_success("Subdirectories already exists");
                    } else if err.kind() == std::io::ErrorKind::NotFound {
                        crate::services::logger::log_error("Parent directory not found", &err);
                    } else {
                        crate::services::logger::log_error("couldnt create subdirectory", &err);
                    }
                }
            };
        }
        crate::services::logger::log_success("file paths created, app directory created");
    }
    ProgramFiles {
        base: user_home_path.clone(),
        data_base_path: user_home_path.join("note.sqlite"),
        notes_path: user_home_path.join("notes"),
        assets_path: user_home_path.join("assets"),
        logs_path: user_home_path.join("logs/app.log"),
        keys_path: user_home_path.join("keys/master.key"),
        config_path: user_home_path.join("config.json"),
        tmp_path: user_home_path.join("tmp"),
        delete_tmp_path: user_home_path.join("tmp_delete"),
        local_login_database_path: program_home_path.join("smartnote/users/local_login_db.sqlite"),
        device_id_path: program_home_path.join("smartnote/device_id.json"),
        active_user_path: program_home_path.join("smartnote/active_user.json"),
    }
}

///function responsible for writeing config data, current directory and if is it fallback
fn write_config(fallback: bool, program_paths: &ProgramFiles) {
    let config_content = ConfigData {
        fallback: fallback,
        data_dir: program_paths.base.to_path_buf(),
    };
    let serialized_config_content = serde_json::to_string_pretty(&config_content); //pretty
    let content: String = match serialized_config_content {
        Ok(ser_config_content) => {
            crate::services::logger::log_success("serialized config content");
            ser_config_content
        }
        Err(err) => {
            crate::services::logger::log_error(
                "serializing error config will be empty string, error",
                err,
            );
            "{}".to_string()
        }
    };
    match fs::write(&program_paths.config_path, &content) {
        Ok(_) => {
            crate::services::logger::log_success("written content to config.json");
        }
        Err(err) => {
            fs::write(program_paths.config_path.clone(), "{}").ok(); //it can
            crate::services::logger::log_error(
                "couldnt write to config.json, it will try again to write only braccets or if it fail it will have no content or not existing",
                err,
            );
        }
    }
}

/// function for getting device id, or creating new if not exists
pub fn get_device_id(paths: &ProgramFiles) -> Result<uuid::Uuid, crate::errors::Error> {
    if paths.device_id_path.exists() {
        let file_content = std::fs::read_to_string(&paths.device_id_path)?;
        let parsed_file: serde_json::Value = serde_json::from_str(&file_content)?;
        let device_id = uuid::Uuid::parse_str(
            parsed_file["device_id"]
                .as_str()
                .expect("device_id must be a string UUID in config file"),
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
            .expect("device_id must be a string UUID in config file"),
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
