use dirs_next::data_local_dir;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
#[derive(Debug)]
pub struct ProgramFiles {
    pub base: PathBuf,
    pub data_base_path: PathBuf,
    pub notes_path: PathBuf,
    pub assets_path: PathBuf,
    pub logs_path: PathBuf,
    pub keys_path: PathBuf,
    pub config_path: PathBuf,
}
#[derive(Serialize, Deserialize)]
pub struct ConfigData {
    pub fallback: bool,
    pub data_dir: PathBuf,
}

impl ProgramFiles {
    pub fn init() -> ProgramFiles {
        let mut fallback = false;
        let base = data_local_dir();

        match base {
            Some(program_home_path) => {
                let program_paths = get_paths(program_home_path);
                write_config(fallback, &program_paths);
                program_paths
            }
            None => {
                fallback = true;
                let log_content = "Couldnt get directory, trying to fallback";
                crate::services::logger::log_error(log_content, "");
                let first_fallback = std::env::temp_dir(); //cleaner logic needed when it will be added
                let program_paths = get_paths(first_fallback);
                write_config(fallback, &program_paths);
                program_paths
            }
        }
    }
}
pub fn get_paths(mut program_home_path: PathBuf) -> ProgramFiles {
    let user = String::from("user1");
    let app_string = format!("smartnote/users/{}/", user); //in the future add here custom username gettn from login
    program_home_path.push(app_string);
    if let Err(err) = std::fs::create_dir_all(&program_home_path) {
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
        for path in ["notes", "assets", "keys", "logs"] {
            let path_to_create = program_home_path.join(path);
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
        base: program_home_path.clone(),
        data_base_path: program_home_path.join("note.sqlite"),
        notes_path: program_home_path.join("notes"),
        assets_path: program_home_path.join("assets"),
        logs_path: program_home_path.join("logs/app.log"),
        keys_path: program_home_path.join("keys/master.key"),
        config_path: program_home_path.join("config.json"),
    }
}
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
#[test]
fn init_test() {
    let paths = ProgramFiles::init();
    println!("{:?}", paths);
    assert!(paths.config_path.exists())
}
