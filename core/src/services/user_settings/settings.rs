use crate::{
    ProgramFiles,
    services::{
        self,
        user_settings::settings_constants::{SECTIONS_META, SETTINGS_META},
    },
    utils::log_helper,
};
use anyhow::Context;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum SettingInputType {
    Switch,
    Button,
    Select,
    Number,
    Info,
}
//Structs for sending cosnfig to frontend
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Setting {
    pub id: String,
    pub setting_name: String,
    pub label: String,
    pub description: String,
    pub current_value: String,
    pub input_type: SettingInputType,
    pub options: Option<Vec<String>>,
    pub button_label: Option<String>,
}

impl Setting {
    pub fn new(
        id: String,
        setting_name: String,
        label: String,
        description: String,
        current_value: String,
        input_type: SettingInputType,
        button_label: Option<String>,
    ) -> Setting {
        return Setting {
            id,
            setting_name,
            label,
            description,
            current_value,
            input_type,
            options: None,
            button_label,
        };
    }
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Section {
    pub id: String,
    pub subsections: Option<Vec<Section>>,
    pub section_name: String,
    pub section_settings: Vec<Setting>,
}

impl Section {
    pub fn new(
        id: String,
        name: String,
        subsections: Option<Vec<Section>>,
        section_settings: Vec<Setting>,
    ) -> Section {
        return Section {
            id,
            subsections,
            section_name: name,
            section_settings,
        };
    }
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UserConfig {
    pub sections: Vec<Section>,
}
//CONFIGS FOR WRITING INTO config.json file
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WriteSection {
    pub section_id: String,
    pub write_sections: Option<Vec<WriteSection>>,
    pub settings: IndexMap<String, String>,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WriteConfig {
    pub sections: Vec<WriteSection>,
}

pub fn get_config(paths: &ProgramFiles) -> Result<(UserConfig, bool), crate::errors::Error> {
    let config_content = std::fs::read_to_string(paths.config_path.clone());
    match config_content {
        Ok(content) => {
            log_helper(
                "getting config",
                "success",
                None::<crate::utils::Format<String>>,
                "successfully readed users config",
            );

            if content.trim().is_empty() {
                return Ok((write_default_config(paths)?, true));
            }

            let write_config: Result<WriteConfig, anyhow::Error> =
                serde_json::from_str(&content).context("failed to deserialize user config");
            match write_config {
                Ok(write_config) => {
                    let user_config: UserConfig = parse_write_to_user_config(write_config);
                    return Ok((user_config, false));
                }
                Err(_err) => {
                    return Ok((write_default_config(paths)?, true));
                }
            }
        }
        Err(err) => {
            tracing::error!(
                task = "getting config",
                status = "erorr",
                error = ?err,
                "Error while reading config, fallback creation of default config"
            );
            // if paths.config_path.exists() {
            //     std::fs::copy(&paths.config_path, &paths.config_backup_path)
            //         .context("Failed to copy config to backup")?;
            // }
            return Ok((write_default_config(paths)?, true));
        }
    }
}

fn fallback_create_default_state(
    paths: &ProgramFiles,
    err_msg: &str,
) -> Result<IndexMap<String, String>, crate::errors::Error> {
    tracing::error!(
        task = "getting config",
        status = "error",
        error = %err_msg,
        "Error while reading config, fallback creation of default config"
    );
    write_default_config(paths)?;
    Ok(get_config_for_state(paths)?)
}
pub fn get_config_for_state(
    paths: &ProgramFiles,
) -> Result<IndexMap<String, String>, crate::errors::Error> {
    let config_content = std::fs::read_to_string(paths.config_path.clone());
    match config_content {
        Ok(content) => {
            log_helper(
                "getting config",
                "success",
                None::<crate::utils::Format<String>>,
                "successfully readed users config",
            );
            if content.trim().is_empty() {
                write_default_config(paths)?;
                return Ok(get_config_for_state(paths)?);
            }
            let user_config: Result<WriteConfig, anyhow::Error> =
                serde_json::from_str(&content).context("failed to deserialize user config");
            match user_config {
                Ok(user_config) => {
                    if count_settings(&user_config.sections)
                        != crate::services::user_settings::settings_constants::NUMBER_OF_SETTINGS
                    {
                        return fallback_create_default_state(
                            paths,
                            &format!(
                                "Wrong config structure, more or less settings in config.json",
                            ),
                        );
                    }

                    let state_config: IndexMap<String, String> =
                        parse_config_to_state_index_map(&user_config);
                    if check_config_correctnes(&state_config) {
                        // std::fs::copy(&paths.config_path, &paths.config_backup_path)
                        //     .context("Failed to copy config to backup")?;
                        Ok(state_config)
                    } else {
                        fallback_create_default_state(
                            paths,
                            &format!("Config not correct, some setting are missing"),
                        )
                    }
                }

                Err(err) => return fallback_create_default_state(paths, &format!("{:?}", err)),
            }
        }
        Err(err) => {
            return fallback_create_default_state(paths, &format!("{:?}", err));
        }
    }
}
fn write_default_config(paths: &ProgramFiles) -> Result<UserConfig, crate::errors::Error> {
    let default_config = crate::services::user_settings::settings_constants::default_config(
        paths
            .base
            .clone()
            .to_str()
            .context("failed to parse path to str")?,
    );
    let parsed_config = parse_config(&default_config);
    let config_content = serde_json::to_string_pretty(&parsed_config)
        .inspect_err(|err| {
            tracing::error!(
                task = "getting config",
                status = "error",
                error = ?err,
                "Error while reading config, fallback creation of default config"
            );
        })
        .context("failed to parse UserConfig to json")?;
    std::fs::write(&paths.config_path, config_content)?;
    Ok(default_config)
}

fn parse_config(config: &UserConfig) -> WriteConfig {
    let mut sections_vec: Vec<WriteSection> = Vec::new();
    for section in &config.sections {
        sections_vec.push(parse_section_recursevly(&section));
    }
    return WriteConfig {
        sections: sections_vec,
    };
}

fn parse_section_recursevly(section: &Section) -> WriteSection {
    let section_id = section.id.clone();

    let mut settings_map = IndexMap::new();
    for setting in &section.section_settings {
        settings_map.insert(setting.id.clone(), setting.current_value.clone());
    }

    let write_sections = if let Some(subsections) = &section.subsections {
        let mut write_subsections = Vec::new();
        for subsection in subsections {
            write_subsections.push(parse_section_recursevly(subsection));
        }
        Some(write_subsections)
    } else {
        None
    };

    WriteSection {
        section_id,
        write_sections,
        settings: settings_map,
    }
}
fn parse_write_to_user_config(write_config: WriteConfig) -> UserConfig {
    let mut sections: Vec<Section> = Vec::new();

    for write_section in write_config.sections {
        sections.push(parse_write_sections_recursevly(write_section));
    }

    UserConfig { sections }
}

fn parse_write_sections_recursevly(section: WriteSection) -> Section {
    if let Some(subsection_vec) = section.write_sections {
        let mut return_subsection_vec: Vec<Section> = Vec::new();
        for subsection in subsection_vec {
            return_subsection_vec.push(parse_write_sections_recursevly(subsection));
        }
        let section_meta = SECTIONS_META.get(&section.section_id).unwrap();
        let write_settings = section.settings;
        let section_settings = parse_settings(write_settings);
        return Section {
            id: section.section_id,
            subsections: Some(return_subsection_vec),
            section_name: section_meta.label.to_string(),
            section_settings: section_settings,
        };
    } else {
        let section_meta = SECTIONS_META.get(&section.section_id).unwrap(); //unwrap because section names are hardcoded
        let write_settings = section.settings;
        let section_settings = parse_settings(write_settings);
        return Section {
            id: section.section_id,
            subsections: None,
            section_name: section_meta.label.to_string(),
            section_settings: section_settings,
        };
    }
}

fn parse_settings(settings: IndexMap<String, String>) -> Vec<Setting> {
    let mut return_settings: Vec<Setting> = Vec::new();
    for (key, value) in settings {
        let setting_meta = SETTINGS_META.get(&key).unwrap(); //unwrap because values are hardcoded 
        return_settings.push(Setting {
            id: key,
            setting_name: setting_meta.field.to_string(),
            label: setting_meta.label.to_string(),
            description: setting_meta.description.to_string(),
            current_value: value,
            input_type: setting_meta.input_type,
            options: setting_meta
                .options
                .map(|o| o.iter().map(|s| s.to_string()).collect()),
            button_label: setting_meta.button_label.map(|s| s.to_string()),
        })
    }
    return_settings
}

fn parse_config_to_state_index_map(readed_config: &WriteConfig) -> IndexMap<String, String> {
    let mut return_map = IndexMap::new();
    for section in &readed_config.sections {
        return_map.extend(handle_write_sections(section));
    }
    return_map
}

fn handle_write_sections(section: &WriteSection) -> IndexMap<String, String> {
    let mut collect_map: IndexMap<String, String> = IndexMap::new();
    collect_map.extend(section.settings.clone());
    if let Some(subsection) = &section.write_sections {
        for s in subsection {
            collect_map.extend(handle_write_sections(&s));
        }
    }
    collect_map
}

pub fn save_config(
    config: &UserConfig,
    config_path: PathBuf,
    config_path_backup: PathBuf,
) -> Result<IndexMap<String, String>, crate::errors::Error> {
    let parsed_config = parse_config(&config);
    let config_content = serde_json::to_string_pretty(&parsed_config)
        .inspect_err(|err| {
            tracing::error!(
                task = "getting config",
                status = "error",
                error = ?err,
                "Error while parsing changed config"
            );
        })
        .context("failed to parse UserConfig to json")?;
    let hash_config = parse_config_to_state_index_map(&parsed_config);
    std::fs::copy(&config_path, config_path_backup).context("Failed to copy config to backup")?;
    std::fs::write(&config_path, config_content)?;
    Ok(hash_config)
}

fn count_settings(write_sections: &Vec<WriteSection>) -> i64 {
    let mut counter: i64 = 0;
    for section in write_sections {
        counter += section.settings.len() as i64;
        if let Some(subsections) = &section.write_sections {
            counter += count_settings(subsections)
        }
    }
    return counter;
}
fn check_config_correctnes(settings_map: &IndexMap<String, String>) -> bool {
    let expected = services::user_settings::settings_constants::SETTING_NAME_LIST;
    if settings_map.len() != expected.len() {
        return false;
    }
    for &name in expected.iter() {
        if !settings_map.contains_key(name) {
            return false;
        }
    }
    true
}
pub fn load_config_backup(
    backup_path: &PathBuf,
    config_path: &PathBuf,
) -> Result<(), crate::errors::Error> {
    std::fs::copy(backup_path, config_path)?;
    Ok(())
}
//change state hashmap, change hashmap on frontend
#[test]

fn state_index_map_test() {
    let write_config = parse_config(
        &crate::services::user_settings::settings_constants::default_config(
            "C:\\Users\\Jakub\\AppData\\Local\\llava/users/ffcd2a2c-2de2-4864-9b8c-326e240bf385/",
        ),
    );
    let parsed = parse_config_to_state_index_map(&write_config);
    println!("{:#?}", parsed);
    assert_eq!(
        parsed.len(),
        services::user_settings::settings_constants::NUMBER_OF_SETTINGS as usize
    );
}

#[test]
fn detect_duplicate_settings_by_length() {
    // Create a WriteConfig with duplicated setting keys across sections
    let mut s1 = IndexMap::new();
    s1.insert("local.mode".to_string(), "true".to_string());

    let mut s2 = IndexMap::new();
    // duplicate key intentionally
    s2.insert("local.mode".to_string(), "false".to_string());

    let section1 = WriteSection {
        section_id: "local".to_string(),
        write_sections: None,
        settings: s1,
    };

    let section2 = WriteSection {
        section_id: "local_sub".to_string(),
        write_sections: None,
        settings: s2,
    };

    let write_config = WriteConfig {
        sections: vec![section1, section2],
    };

    let total_count = count_settings(&write_config.sections) as usize;
    let parsed = parse_config_to_state_index_map(&write_config);
    let unique_count = parsed.len();

    // If duplicates exist, total_count (sum of all settings) will be greater than unique_count
    assert!(
        total_count > unique_count,
        "Expected duplicate detection by differing lengths"
    );
}

#[test]
fn parse_and_check_defaults_no_duplicates() {
    let write_config = parse_config(
        &crate::services::user_settings::settings_constants::default_config(
            "C:\\Users\\Jakub\\AppData\\Local\\llava/users/ffcd2a2c-2de2-4864-9b8c-326e240bf385/",
        ),
    );
    let parsed = parse_config_to_state_index_map(&write_config);
    // Ensure no duplicates and that keys match expected set
    assert_eq!(
        parsed.len(),
        services::user_settings::settings_constants::NUMBER_OF_SETTINGS as usize
    );
    assert!(check_config_correctnes(&parsed));
}

#[test]
fn deserialize_write_config_preserves_settings_order() {
    let raw = r#"
        {
            "sections": [
                {
                    "sectionId": "local.core",
                    "writeSections": null,
                    "settings": {
                        "local.importNotes": "idle",
                        "local.mode": "off",
                        "local.encryption": "on"
                    }
                }
            ]
        }
        "#;

    let parsed: WriteConfig =
        serde_json::from_str(raw).expect("WriteConfig should deserialize from JSON");
    let keys: Vec<String> = parsed.sections[0].settings.keys().cloned().collect();

    assert_eq!(
        keys,
        vec![
            "local.importNotes".to_string(),
            "local.mode".to_string(),
            "local.encryption".to_string()
        ]
    );
}
