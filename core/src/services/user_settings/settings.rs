use std::collections::HashMap;

use anyhow::Context;
use serde::{Deserialize, Serialize};
//TODO add here camel case #[serde(rename_all = "camelCase")] before structs
use crate::{
    ProgramFiles,
    services::user_settings::settings_constants::{SECTIONS_META, SETTINGS_META, default_config},
    utils::log_helper,
};

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum SettingInputType {
    Switch,
    Button,
    Text,
    Select,
    Number,
    Info,
}
//Structs for sending config to frontend
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Setting {
    pub id: String,
    pub setting_name: String,
    pub label: String,
    pub description: String,
    pub current_value: String,
    pub input_type: SettingInputType,
}

impl Setting {
    pub fn new(
        id: String,
        setting_name: String,
        label: String,
        description: String,
        current_value: String,
        input_type: SettingInputType,
    ) -> Setting {
        return Setting {
            id,
            setting_name,
            label,
            description,
            current_value,
            input_type,
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
    pub settings: std::collections::HashMap<String, String>,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WriteConfig {
    pub sections: Vec<WriteSection>,
}

pub fn get_config(paths: &ProgramFiles) -> Result<UserConfig, crate::errors::Error> {
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
                return Ok(write_default_config(paths)?);
            } //TODO add here checking if all config sections and settings are written, if not, write them as default | flat settings and iterate, same with sections
            let json: serde_json::Value =
                serde_json::from_str(&content).context("Failed to serialize config content")?;
            let user_config: WriteConfig =
                serde_json::from_value(json).context("failed to deserialize user config")?;
            let user_config: UserConfig = parse_write_to_user_config(user_config);
            println!("{:?}", user_config);
            Ok(user_config)
        }
        Err(err) => {
            tracing::error!(
                task = "getting config",
                status = "erorr",
                error= ?err,
                "Error while reading config, fallback creation of default config"
            );
            Ok(write_default_config(paths)?)
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

// TODO write parse config fucntion again, but recursively its much easier, think about mapping settings from iterator to hashMap

fn parse_section_recursevly(section: &Section) -> WriteSection {
    let section_id = section.id.clone();

    let mut settings_map = std::collections::HashMap::new();
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

fn parse_settings(settings: HashMap<String, String>) -> Vec<Setting> {
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
        })
    }
    return_settings
}
