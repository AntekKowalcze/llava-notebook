use phf_macros::phf_map;
use std::collections::{HashMap, HashSet};
//Metaphone algoritm index, update on setting content change
pub static PHONETIC_CORPUS: phf::Map<&'static str, &'static [&'static str]> = phf_map! {
    "local.mode" => &["local", "offline", "mode", "device", "work"],
    "local.encryption" => &["encrypt", "encryption", "secure", "security", "protect", "protection", "private", "privacy", "data", "notes"],
    "local.sync" => &["sync", "synchronize", "synchronization", "background", "local"],
    "local.showLogs" => &["logs", "log", "show", "view", "application", "debug", "recent", "activity", "diagnostic"],
    "local.exportNotes" => &["export", "notes", "backup", "file", "save", "download", "extract", "output"],
    "local.importNotes" => &["import", "notes", "backup", "file", "restore", "load", "upload", "recover"],
    "local.autoBackup" => &["automatic", "auto", "backup", "backups", "schedule", "scheduled", "notes"],
    "local.backupFrequency" => &["frequency", "backup", "interval", "schedule", "daily", "weekly", "monthly", "repeat", "period"],
    "local.dataDirectory" => &["data", "directory", "folder", "path", "location", "database", "files", "storage"],
    "local.loadConfigBackup" => &["load", "backup", "config", "restore", "edited", "broken", "corrupt", "file", "recover", "repair", "last", "working", "version"],
    "local.logout" => &["logout", "signout", "sign", "account", "session", "leave", "exit"],
    "local.deleteLocalFiles" => &["delete", "local", "files", "remove", "permanent", "wipe", "clear", "purge", "erase", "reset"],
    "local.deleteAccount" => &["delete", "account", "remove", "permanent", "wipe", "purge", "erase", "synced"],
    "local.changeUsername" => &["username", "name", "change", "update", "account", "profile", "rename", "set"],
    "local.changePassword" => &["password", "change", "local", "update", "credential", "security", "reset"],
    "online.mode" => &["online", "mode", "connect", "connected", "cloud", "account", "network"],
    "online.sync" => &["sync", "synchronize", "notes", "devices", "automatic", "cross", "connected", "cloud"],
    "online.connectedDevices" => &["connected", "devices", "manage", "view", "account", "link", "linked"],
    "online.changePasswordEmail" => &["password", "email", "change", "reset", "link", "send", "security", "credential"],
    "online.changeUsername" => &["username", "name", "change", "update", "account", "profile", "rename"],
    "online.aiSummary" => &["ai", "summary", "summaries", "generate", "notes", "intelligence", "automation", "smart"],
};

pub fn create_metaphone_map() -> HashMap<String, Vec<String>> {
    // use a temporary map to dedupe entries using HashSet
    let mut temp: HashMap<String, HashSet<String>> = HashMap::new();

    for (key, value) in PHONETIC_CORPUS.entries() {
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
        if p != c || c == 'c' {
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
                        'k' => {
                            output.push('k');
                            index += 2;
                            continue;
                        }

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

            _ => {
                output.push(chars_arr[index]);
                index += 1;
            }
        } //for sure something because it iterates over length
    }

    output
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
fn check_real_usecases() {
    assert_eq!(metaphone("delete"), "tlt");
    assert_eq!(metaphone("local"), "lkl");
    assert_eq!(metaphone("encrypt"), "enkrpt");
    assert_eq!(metaphone("logs"), "lks");
    assert_eq!(metaphone("password"), "pswrt");
    assert_eq!(metaphone("sync"), "snc");
    assert_eq!(metaphone("export"), "eksprt");
    assert_eq!(metaphone("ai"), "a");
}

#[test]

fn print_phonetic_corpus_metaphones() {
    for (key, words) in PHONETIC_CORPUS.entries() {
        println!("# {}", key);
        for &w in *words {
            println!("{} -> {}", w, metaphone(w));
        }
    }
}

#[test]
fn see_real_corpus() {
    let map = create_metaphone_map();
    println!("{:#?}", map);
}
