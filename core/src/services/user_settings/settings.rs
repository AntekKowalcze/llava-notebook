use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::{ProgramFiles, utils::log_helper};
//Structs for sending config to frontend
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Setting {
    pub id: String,
    pub setting_name: String,
    pub label: String,
    pub description: String,
    pub current_value: String,
}

impl Setting {
    pub fn new(
        id: String,
        setting_name: String,
        label: String,
        description: String,
        current_value: String,
    ) -> Setting {
        return Setting {
            id,
            setting_name,
            label,
            description,
            current_value,
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

pub struct UserConfig {
    pub sections: Vec<Section>,
}
//CONFIGS FOR WRITING INTO config.json file
#[derive(Deserialize, Serialize, Debug)]

pub struct WriteSection {
    pub section_id: String,
    pub write_sections: Option<Vec<WriteSection>>,
    pub settings: std::collections::HashMap<String, String>,
}

#[derive(Deserialize, Serialize, Debug)]
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
            if content == "" {
                return Ok(write_default_config(paths)?);
            }
            let json: serde_json::Value =
                serde_json::from_str(&content).context("Failed to serialize config content")?;
            let user_config: UserConfig =
                serde_json::from_value(json).context("failed to deserialize user config")?;

            Ok(user_config)
        }
        Err(err) => {
            tracing::error!(
                task = "getting config",
                status = "erorr",
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
// TODO write parse config fucntion again, but recursively its much easier, think about mapping settings from iterator to hashMap
