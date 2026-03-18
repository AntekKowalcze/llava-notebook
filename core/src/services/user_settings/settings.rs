use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use crate::{
    ProgramFiles,
    services::user_settings::settings_constants::{SECTIONS_META, SETTINGS_META},
    utils::log_helper,
};
use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum SettingInputType {
    Switch,
    Button,
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
            }
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
pub fn get_config_for_state(
    paths: &ProgramFiles,
) -> Result<std::collections::HashMap<String, String>, crate::errors::Error> {
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
            //TODO delete checking in get_config, do it here (earlier in program lifetime) (frontend version) add here checking if all config sections and settings are written, if not, write them as default | flat settings and iterate, same with sections
            let json: serde_json::Value =
                serde_json::from_str(&content).context("Failed to serialize config content")?;
            let user_config: WriteConfig =
                serde_json::from_value(json).context("failed to deserialize user config")?;
            let state_config: std::collections::HashMap<String, String> =
                parse_config_to_state_hash_map(user_config);
            Ok(state_config)
        }
        Err(err) => {
            tracing::error!(
                task = "getting config",
                status = "erorr",
                error= ?err,
                "Error while reading config, fallback creation of default config"
            );
            write_default_config(paths)?;
            return Ok(get_config_for_state(paths)?);
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
            options: setting_meta
                .options
                .map(|o| o.iter().map(|s| s.to_string()).collect()),
            button_label: setting_meta.button_label.map(|s| s.to_string()),
        })
    }
    return_settings
}

fn parse_config_to_state_hash_map(
    readed_config: WriteConfig,
) -> std::collections::HashMap<String, String> {
    let mut return_map = std::collections::HashMap::new();
    for section in readed_config.sections {
        return_map.extend(handle_write_sections(section));
    }
    return_map
}

fn handle_write_sections(section: WriteSection) -> std::collections::HashMap<String, String> {
    let mut collect_map: HashMap<String, String> = HashMap::new();
    collect_map.extend(section.settings);
    if let Some(subsection) = section.write_sections {
        for s in subsection {
            collect_map.extend(handle_write_sections(s));
        }
    }
    collect_map
}

pub fn save_config(
    config: &UserConfig,
    config_path: PathBuf,
) -> Result<std::collections::HashMap<String, String>, crate::errors::Error> {
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
    let hash_config = parse_config_to_state_hash_map(parsed_config);
    std::fs::write(config_path, config_content)?;
    Ok(hash_config)
}
fn create_metaphone_map() -> HashMap<String, Vec<String>> {
    // use a temporary map to dedupe entries using HashSet
    let mut temp: HashMap<String, HashSet<String>> = HashMap::new();

    for (key, value) in
        crate::services::user_settings::settings_constants::PHONETIC_CORPUS.entries()
    {
        // normalize key: remove dots and non-alphanumeric characters before metaphoning
        let normalized_key: String = key.chars().filter(|c| c.is_alphanumeric()).collect();
        let metaphone_key = metaphone(&normalized_key);
        if !metaphone_key.is_empty() {
            temp.entry(metaphone_key)
                .or_insert_with(HashSet::new)
                .insert(key.to_string());
        }

        for &word in *value {
            // normalize corpus word similarly for metaphone computation
            let normalized_word: String = word.chars().filter(|c| c.is_alphanumeric()).collect();
            let m_key = metaphone(&normalized_word);
            if !m_key.is_empty() {
                temp.entry(m_key)
                    .or_insert_with(HashSet::new)
                    .insert(key.to_string());
            }
        }
    }

    // convert HashSet values back to Vec<String>
    let mut return_map: HashMap<String, Vec<String>> = HashMap::new();
    for (k, set) in temp {
        let mut v: Vec<String> = set.into_iter().collect();
        v.sort();
        return_map.insert(k, v);
    }

    return_map
}

fn metaphone(entry: &str) -> String {
    let entry = entry.trim().to_lowercase();
    let vovel_arr = ['a', 'e', 'i', 'o', 'u'];
    let mut output = String::new();

    let mut entry_worker = String::new();

    if let Some(first_char) = entry.chars().next() {
        entry_worker.push(first_char);
    }

    for (p, c) in entry.chars().zip(entry.chars().skip(1)) {
        if p != c && p != 'c' {
            entry_worker.push(c);
        }
    }

    if entry_worker.starts_with("kn")
        || entry_worker.starts_with("gn")
        || entry_worker.starts_with("pn")
        || entry_worker.starts_with("ae")
        || entry_worker.starts_with("wr")
    {
        entry_worker = entry_worker.chars().skip(1).collect();
    }

    if entry_worker.ends_with("mb") {
        entry_worker.pop();
    }

    if let Some(prefix) = entry_worker.strip_suffix("gned") {
        entry_worker = format!("{}{}", prefix, "ned");
    } else if let Some(prefix) = entry_worker.strip_suffix("gn") {
        entry_worker = format!("{}{}", prefix, "n");
    } else if entry_worker.ends_with('g') {
        entry_worker.pop();
    }

    let chars_arr: Vec<char> = entry_worker.chars().collect();
    let mut index = 0;
    while index < chars_arr.len() {
        match chars_arr[index] {
            's' => {
                // s before c to catch sch before matching 'c'
                if index + 2 < chars_arr.len() {
                    if chars_arr[index + 1] == 'c' && chars_arr[index + 2] == 'h' {
                        output.push('k');
                        index += 3;
                        continue;
                    }
                    if chars_arr[index + 1] == 'i'
                        && (chars_arr[index + 2] == 'o' || chars_arr[index + 2] == 'a')
                    {
                        output.push('x');
                        index += 3;
                        continue;
                    }
                }
                if index + 1 < chars_arr.len() {
                    if chars_arr[index + 1] == 'h' {
                        output.push('x');
                        index += 2;
                        continue;
                    }
                }

                output.push('s');
                index += 1;
                continue;
            }
            't' => {
                if index + 2 < chars_arr.len() {
                    if chars_arr[index + 1] == 'i'
                        && (chars_arr[index + 2] == 'o' || chars_arr[index + 2] == 'a')
                    {
                        output.push('x');
                        index += 3;
                        continue;
                    }

                    if chars_arr[index + 1] == 'c' && chars_arr[index + 2] == 'h' {
                        index += 3;
                        continue;
                    }
                }

                if index + 1 < chars_arr.len() {
                    if chars_arr[index + 1] == 'h' {
                        output.push('0');
                        index += 2;
                        continue;
                    }
                }
                output.push('t');
                index += 1;
            }

            'p' => {
                if index + 1 < chars_arr.len() {
                    if chars_arr[index + 1] == 'h' {
                        output.push('f');
                        index += 2;
                        continue;
                    }
                }

                output.push('p');
                index += 1;
                continue;
            }

            'k' => {
                if index > 0 {
                    if chars_arr[index - 1] == 'c' {
                        output.push('k');
                        index += 1;
                        continue;
                    }
                }

                output.push('k');
                index += 1;
                continue;
            }
            'c' => {
                if index + 2 < chars_arr.len() {
                    if chars_arr[index + 1] == 'i' && chars_arr[index + 2] == 'a' {
                        output.push('x');
                        index += 3;
                        continue;
                    }
                }
                if index < chars_arr.len() - 1 {
                    match chars_arr[index + 1] {
                        'h' => {
                            output.push('x');
                            index += 2;
                            continue;
                        }
                        'i' | 'e' | 'y' => {
                            output.push('s');
                            index += 2
                        }
                        _ => {
                            output.push('k');
                            index += 1
                        }
                    }
                } else {
                    output.push(chars_arr[index]);
                    index += 1;
                }
            }
            'd' => {
                if index + 2 < chars_arr.len() {
                    if chars_arr[index + 1] == 'g'
                        && (chars_arr[index + 2] == 'e'
                            || chars_arr[index + 2] == 'y'
                            || chars_arr[index + 2] == 'i')
                    {
                        output.push('j');
                        index += 3;
                        continue;
                    } else {
                        output.push('t');
                        index += 1;
                        continue;
                    }
                } else {
                    output.push('t');
                    index += 1;
                    continue;
                }
            }
            'g' => {
                if index + 1 < chars_arr.len() {
                    if chars_arr[index + 1] == 'h'
                        && index + 2 < chars_arr.len()
                        && !vovel_arr.contains(&chars_arr[index + 2])
                    {
                        index += 1;
                        continue;
                    }

                    if (chars_arr[index + 1] == 'i'
                        || chars_arr[index + 1] == 'e'
                        || chars_arr[index + 1] == 'y')
                        && (index == 0 || chars_arr[index - 1] != 'g')
                    {
                        output.push('j');
                        index += 2;
                        continue;
                    }

                    output.push('k');
                    index += 1;
                    continue;
                } //should g be dropped at the end?

                output.push('k');
                index += 1;
                continue;
            }
            'h' => {
                if index > 0 && index + 1 < chars_arr.len() {
                    if vovel_arr.contains(&chars_arr[index - 1])
                        && !vovel_arr.contains(&chars_arr[index + 1])
                    {
                        index += 1;
                        continue;
                    } else {
                        output.push('h');
                        index += 1;
                        continue;
                    }
                }

                index += 1;
                continue;
            }
            'q' => {
                output.push('k');
                index += 1;
                continue;
            }
            'v' => {
                output.push('f');
                index += 1;
                continue;
            }
            'w' => {
                if index == 0 && index + 1 < chars_arr.len() && chars_arr[index + 1] == 'h' {
                    output.push('w');
                    index += 2;
                    continue;
                }
                if index + 1 < chars_arr.len() {
                    if vovel_arr.contains(&chars_arr[index + 1]) {
                        output.push('w');
                        index += 1;
                        continue;
                    } else {
                        index += 1;
                        continue;
                    }
                }

                index += 1;
                continue;
            }
            'x' => {
                if index == 0 {
                    output.push('s');
                    index += 1;
                    continue;
                } else {
                    output.push('k');
                    output.push('s');
                    index += 1;
                    continue;
                }
            }
            'y' => {
                if index + 1 < chars_arr.len() {
                    if vovel_arr.contains(&chars_arr[index + 1]) {
                        output.push('y');
                        index += 1;
                        continue;
                    } else {
                        index += 1;
                        continue;
                    }
                }

                index += 1;
                continue;
            }
            'z' => {
                output.push('s');
                index += 1;
                continue;
            }
            'a' | 'e' | 'i' | 'u' | 'o' => {
                if index == 0 {
                    output.push(chars_arr[index]);
                    index += 1;
                    continue;
                }
                index += 1;
                continue;
            }

            _ => index += 1,
        } //for sure something because it iterates over length
    }

    output
}

#[test]

fn state_hash_map_test() {
    let write_config = parse_config(
        &crate::services::user_settings::settings_constants::default_config(
            "C:\\Users\\Jakub\\AppData\\Local\\llava/users/ffcd2a2c-2de2-4864-9b8c-326e240bf385/",
        ),
    );
    let parsed = parse_config_to_state_hash_map(write_config);
    println!("{:#?}", parsed);
    assert_eq!(parsed.len(), 18); //change value depending on number of setting on the list!!
}
#[test]
fn test_metaphone() {
    assert_eq!(metaphone("test"), "tst");
}

#[test]
fn test_metaphone_short_inputs_do_not_panic() {
    assert_eq!(metaphone(""), "");
    assert_eq!(metaphone("a"), "a");
    assert_eq!(metaphone("x"), "s");
    assert_eq!(metaphone("v"), "f");
}

#[test]
fn test_metaphone_two_char_rules() {
    assert_eq!(metaphone("sh"), "x");
    assert_eq!(metaphone("th"), "0");
    assert_eq!(metaphone("ph"), "f");
}

#[test]
fn print_phonetic_corpus_metaphones() {
    use crate::services::user_settings::settings_constants::PHONETIC_CORPUS;
    for (key, words) in PHONETIC_CORPUS.entries() {
        println!("# {}", key);
        for &w in *words {
            println!("{} -> {}", w, metaphone(w));
        }
    }
}

#[test]
fn check_real_usecases() {
    assert_eq!(metaphone("delete"), "tt");
    assert_eq!(metaphone("local"), "k");
    assert_eq!(metaphone("encrypt"), "espt");
    assert_eq!(metaphone("logs"), "ks");
    assert_eq!(metaphone("password"), "pswt");
    assert_eq!(metaphone("sync"), "sc");
    assert_eq!(metaphone("export"), "ekspt");
    assert_eq!(metaphone("ai"), "a");
}

#[test]
fn see_real_corpus() {
    let map = create_metaphone_map();
    println!("{:#?}", map);
}
//TODO impolement methaphone on frontend and add filtration
