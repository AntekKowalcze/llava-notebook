//! # User settings configuration module
//!
//! **Purpose**: This module is responsible for reading, validating, converting, and
//! persisting the user's application settings.
//!
//! It maintains two representations of the configuration:
//! - [`UserConfig`] — UI-facing representation enriched with metadata.
//! - [`WriteConfig`] — file-facing representation stored in `config.json`.
//!
//! The module also provides conversion between the persisted configuration and the
//! runtime [`IndexMap`] used by the backend.
//!
//! ## Exported items
//! * [`SettingInputType`] — Defines the input control type used by the frontend.
//! * [`Setting`] — UI-facing representation of a single setting.
//! * [`Section`] — UI-facing settings section with optional nested subsections.
//! * [`UserConfig`] — Complete UI-facing settings tree.
//! * [`WriteSection`] — File-facing settings section stored in `config.json`.
//! * [`WriteConfig`] — Complete file-facing settings tree.
//! * [`get_config`] — Reads `config.json` and builds a [`UserConfig`]; creates the
//!   default configuration when the file is missing, empty, or invalid.
//! * [`get_config_for_state`] — Reads and validates `config.json`, then builds the
//!   runtime settings [`IndexMap`].
//! * [`save_config`] — Persists a [`UserConfig`] to `config.json` and returns the
//!   resulting runtime settings map.
//! * [`load_config_backup`] — Restores `config.json` from its backup.
//!
//! ## Key design decisions
//! - [`WriteConfig`] is the persistence representation and is stored in `config.json`.
//! - [`UserConfig`] is the frontend representation and is reconstructed from metadata
//!   defined in `settings_constants`.
//! - The runtime settings state is represented by an [`IndexMap`] containing all
//!   settings flattened from the configuration tree.
//! - [`IndexMap`] is used to preserve the order of settings when serialising and
//!   deserialising configuration data.
//! - The configuration is validated against the expected setting list before it is
//!   accepted as runtime state.
//! - Invalid or incomplete configuration files are replaced with the default
//!   configuration.
//! - After saving, the runtime state is rebuilt from the file representation so that
//!   both representations remain consistent.
//!
//! ## Settings synchronisation flow
//! 1. The frontend sends a complete [`UserConfig`] or a settings update.
//! 2. The backend applies the requested change.
//! 3. The backend serialises the configuration to `config.json`.
//! 4. The backend rebuilds the runtime [`IndexMap`] from the persisted representation.
//! 5. The backend emits a configuration update event.
//! 6. The frontend refreshes its state from the updated configuration.
//!
//! ## Dependencies
//! - `serde` / `serde_json` — Serialisation and deserialisation of `config.json`.
//! - `indexmap` — Stable ordering of settings.
//! - `anyhow` — Error context for parsing and filesystem operations.
//! - `tracing` — Structured diagnostic logging.
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

/// UI-facing description of a single application setting.
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
        Setting {
            id,
            setting_name,
            label,
            description,
            current_value,
            input_type,
            options: None,
            button_label,
        }
    }
}

/// UI-facing representation of a settings section.
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
        Section {
            id,
            subsections,
            section_name: name,
            section_settings,
        }
    }
}

/// Complete UI-facing settings tree.
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UserConfig {
    pub sections: Vec<Section>,
}

/// File-facing representation of a settings section.
///
/// Unlike [`Section`], this structure contains only values required to persist
/// the configuration. Display metadata is reconstructed from `settings_constants`.
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WriteSection {
    pub section_id: String,
    pub write_sections: Option<Vec<WriteSection>>,
    pub settings: IndexMap<String, String>,
}

/// Complete file-facing settings tree stored in `config.json`.
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WriteConfig {
    pub sections: Vec<WriteSection>,
}

/// Reads the persisted configuration and converts it into the UI-facing representation.
///
/// If the configuration file is missing, empty, or cannot be deserialised, the default
/// configuration is created and returned.
///
/// # Errors
/// Returns an error if creating or serialising the default configuration fails.
pub fn get_config(paths: &ProgramFiles) -> Result<(UserConfig, bool), crate::errors::Error> {
    tracing::debug!(
        task = "get config",
        path = %paths.config_path.display(),
        "starting configuration read"
    );

    let config_content = std::fs::read_to_string(paths.config_path.clone());

    match config_content {
        Ok(content) => {
            log_helper(
                "get config",
                "success",
                None::<crate::utils::Format<String>>,
                "configuration file read successfully",
            );

            if content.trim().is_empty() {
                tracing::debug!(
                    task = "get config",
                    status = "fallback",
                    "configuration file is empty, creating default configuration"
                );

                return Ok((write_default_config(paths)?, true));
            }

            let write_config: Result<WriteConfig, anyhow::Error> =
                serde_json::from_str(&content).context("failed to deserialize user config");

            match write_config {
                Ok(write_config) => {
                    tracing::debug!(
                        task = "get config",
                        status = "success",
                        "configuration deserialized successfully"
                    );

                    let user_config = parse_write_to_user_config(write_config);

                    Ok((user_config, false))
                }
                Err(err) => {
                    tracing::error!(
                        task = "get config",
                        status = "error",
                        error = ?err,
                        "failed to deserialize configuration, creating default configuration"
                    );

                    Ok((write_default_config(paths)?, true))
                }
            }
        }
        Err(err) => {
            tracing::error!(
                task = "get config",
                status = "error",
                path = %paths.config_path.display(),
                error = ?err,
                "failed to read configuration file, creating default configuration"
            );

            Ok((write_default_config(paths)?, true))
        }
    }
}

fn fallback_create_default_state(
    paths: &ProgramFiles,
    err_msg: &str,
) -> Result<IndexMap<String, String>, crate::errors::Error> {
    tracing::error!(
        task = "get config state",
        status = "fallback",
        error = %err_msg,
        "invalid configuration detected, creating default configuration"
    );

    write_default_config(paths)?;

    tracing::debug!(
        task = "get config state",
        status = "retry",
        "reading runtime state from newly created default configuration"
    );

    get_config_for_state(paths)
}

/// Reads, validates, and flattens the persisted configuration into the runtime state map.
///
/// The configuration must contain exactly the expected number of settings and every
/// required setting key must be present. Invalid configuration data is replaced with
/// the default configuration.
///
/// # Errors
/// Returns an error if the configuration cannot be read or if creating the default
/// configuration fails.
pub fn get_config_for_state(
    paths: &ProgramFiles,
) -> Result<IndexMap<String, String>, crate::errors::Error> {
    tracing::debug!(
        task = "get config state",
        path = %paths.config_path.display(),
        "starting runtime configuration read"
    );

    let config_content = std::fs::read_to_string(paths.config_path.clone());

    match config_content {
        Ok(content) => {
            log_helper(
                "get config state",
                "success",
                None::<crate::utils::Format<String>>,
                "configuration file read successfully",
            );

            if content.trim().is_empty() {
                tracing::debug!(
                    task = "get config state",
                    status = "fallback",
                    "configuration file is empty, creating default configuration"
                );

                write_default_config(paths)?;
                return get_config_for_state(paths);
            }

            let user_config: Result<WriteConfig, anyhow::Error> =
                serde_json::from_str(&content).context("failed to deserialize user config");

            match user_config {
                Ok(user_config) => {
                    let settings_count = count_settings(&user_config.sections);

                    tracing::debug!(
                        task = "get config state",
                        settings_count,
                        expected_settings =
                            crate::services::user_settings::settings_constants::NUMBER_OF_SETTINGS,
                        "configuration structure validated by setting count"
                    );

                    if settings_count
                        != crate::services::user_settings::settings_constants::NUMBER_OF_SETTINGS
                    {
                        return fallback_create_default_state(
                            paths,
                            "wrong configuration structure: unexpected number of settings",
                        );
                    }

                    let state_config = parse_config_to_state_index_map(&user_config);

                    if check_config_correctnes(&state_config) {
                        log_helper(
                            "get config state",
                            "success",
                            Some(crate::utils::Format::Display(&state_config.len())),
                            "configuration validated successfully",
                        );

                        Ok(state_config)
                    } else {
                        fallback_create_default_state(
                            paths,
                            "configuration validation failed: required settings are missing",
                        )
                    }
                }

                Err(err) => fallback_create_default_state(
                    paths,
                    &format!("failed to deserialize configuration: {err:?}"),
                ),
            }
        }
        Err(err) => fallback_create_default_state(
            paths,
            &format!("failed to read configuration file: {err:?}"),
        ),
    }
}

fn write_default_config(paths: &ProgramFiles) -> Result<UserConfig, crate::errors::Error> {
    tracing::debug!(
        task = "write default config",
        path = %paths.config_path.display(),
        "creating default configuration"
    );

    let default_config = crate::services::user_settings::settings_constants::default_config(
        paths
            .base
            .clone()
            .to_str()
            .context("failed to convert base path to string")?,
    );

    let parsed_config = parse_config(&default_config);

    let config_content = serde_json::to_string_pretty(&parsed_config)
        .inspect_err(|err| {
            tracing::error!(
                task = "write default config",
                status = "error",
                error = ?err,
                "failed to serialise default configuration"
            );
        })
        .context("failed to serialise UserConfig to JSON")?;

    std::fs::write(&paths.config_path, config_content)?;

    log_helper(
        "write default config",
        "success",
        None::<crate::utils::Format<String>>,
        "default configuration written successfully",
    );

    Ok(default_config)
}

fn parse_config(config: &UserConfig) -> WriteConfig {
    let mut sections_vec: Vec<WriteSection> = Vec::new();

    for section in &config.sections {
        sections_vec.push(parse_section_recursevly(section));
    }

    WriteConfig {
        sections: sections_vec,
    }
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
    let section_meta = SECTIONS_META
        .get(&section.section_id)
        .expect("section metadata must exist for hardcoded section IDs");

    let section_settings = parse_settings(section.settings);

    let subsections = section.write_sections.map(|subsection_vec| {
        subsection_vec
            .into_iter()
            .map(parse_write_sections_recursevly)
            .collect()
    });

    Section {
        id: section.section_id,
        subsections,
        section_name: section_meta.label.to_string(),
        section_settings,
    }
}

fn parse_settings(settings: IndexMap<String, String>) -> Vec<Setting> {
    let mut return_settings: Vec<Setting> = Vec::new();

    for (key, value) in settings {
        let setting_meta = SETTINGS_META
            .get(&key)
            .expect("setting metadata must exist for hardcoded setting IDs");

        return_settings.push(Setting {
            id: key,
            setting_name: setting_meta.field.to_string(),
            label: setting_meta.label.to_string(),
            description: setting_meta.description.to_string(),
            current_value: value,
            input_type: setting_meta.input_type,
            options: setting_meta
                .options
                .map(|options| options.iter().map(|value| value.to_string()).collect()),
            button_label: setting_meta.button_label.map(|value| value.to_string()),
        });
    }

    return_settings
}

fn parse_config_to_state_index_map(read_config: &WriteConfig) -> IndexMap<String, String> {
    let mut return_map = IndexMap::new();

    for section in &read_config.sections {
        return_map.extend(handle_write_sections(section));
    }

    return_map
}

fn handle_write_sections(section: &WriteSection) -> IndexMap<String, String> {
    let mut collect_map: IndexMap<String, String> = IndexMap::new();

    collect_map.extend(section.settings.clone());

    if let Some(subsections) = &section.write_sections {
        for subsection in subsections {
            collect_map.extend(handle_write_sections(subsection));
        }
    }

    collect_map
}

/// Serialises and persists the supplied UI configuration and returns its runtime state.
///
/// Before overwriting the current configuration, the existing file is copied to the
/// backup path. The returned [`IndexMap`] is rebuilt from the same file-facing
/// representation that is written to disk.
///
/// # Errors
/// Returns an error if the configuration cannot be serialised, the backup cannot be
/// created, or the new configuration cannot be written.
pub fn save_config(
    config: &UserConfig,
    config_path: PathBuf,
    config_path_backup: PathBuf,
) -> Result<IndexMap<String, String>, crate::errors::Error> {
    tracing::debug!(
        task = "save config",
        path = %config_path.display(),
        backup_path = %config_path_backup.display(),
        "starting configuration save"
    );

    let parsed_config = parse_config(config);

    let config_content = serde_json::to_string_pretty(&parsed_config)
        .inspect_err(|err| {
            tracing::error!(
                task = "save config",
                status = "error",
                error = ?err,
                "failed to serialise changed configuration"
            );
        })
        .context("failed to serialise UserConfig to JSON")?;

    let hash_config = parse_config_to_state_index_map(&parsed_config);

    std::fs::copy(&config_path, &config_path_backup).inspect_err(|err| {
        tracing::error!(
            task = "save config",
            status = "error",
            error = ?err,
            "failed to create configuration backup"
        );
    })?;

    std::fs::write(&config_path, config_content).inspect_err(|err| {
        tracing::error!(
            task = "save config",
            status = "error",
            error = ?err,
            "failed to write configuration file"
        );
    })?;

    log_helper(
        "save config",
        "success",
        Some(crate::utils::Format::Display(&hash_config.len())),
        "configuration saved successfully",
    );

    Ok(hash_config)
}

fn count_settings(write_sections: &Vec<WriteSection>) -> i64 {
    let mut counter: i64 = 0;

    for section in write_sections {
        counter += section.settings.len() as i64;

        if let Some(subsections) = &section.write_sections {
            counter += count_settings(subsections);
        }
    }

    counter
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

/// Restores the active configuration from the specified backup file.
///
/// # Errors
/// Returns an error if the backup cannot be copied to the active configuration path.
pub fn load_config_backup(
    backup_path: &PathBuf,
    config_path: &PathBuf,
) -> Result<(), crate::errors::Error> {
    tracing::debug!(
        task = "load config backup",
        backup_path = %backup_path.display(),
        config_path = %config_path.display(),
        "starting configuration backup restore"
    );

    std::fs::copy(backup_path, config_path).inspect_err(|err| {
        tracing::error!(
            task = "load config backup",
            status = "error",
            error = ?err,
            "failed to restore configuration from backup"
        );
    })?;

    log_helper(
        "load config backup",
        "success",
        None::<crate::utils::Format<String>>,
        "configuration backup restored successfully",
    );

    Ok(())
}

#[test]
fn detect_duplicate_settings_by_length() {
    // Create a WriteConfig with duplicated setting keys across sections.
    let mut s1 = IndexMap::new();
    s1.insert("local.mode".to_string(), "true".to_string());

    let mut s2 = IndexMap::new();
    // Duplicate key intentionally.
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

    // If duplicates exist, the total number of stored settings is greater than
    // the number of unique keys in the flattened runtime map.
    assert!(
        total_count > unique_count,
        "Expected duplicate detection by differing lengths"
    );
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
