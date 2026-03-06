use crate::services::user_settings::settings::SettingInputType;
use crate::services::user_settings::settings::{Section, Setting, UserConfig};
//TODO Add buttons so i can preview my dashboard and settings on release, add function to display settings in vue
pub const SETTING_NAME_LIST: &[&str] = &[
    "local.mode",
    "local.encryption",
    "local.sync",
    "local.showLogs",
    "local.exportNotes",
    "local.importNotes",
    "local.autoBackup",
    "local.backupFrequency",
    "local.dataDirectory",
    "local.logout",
    "local.deleteLocalFiles",
    "local.deleteAccount",
    "online.mode",
    "online.sync",
    "online.connectedDevices",
    "online.changePasswordEmail",
    "online.changeUsername",
    "online.aiSummary",
];

pub fn default_config(default_data_dir: &str) -> UserConfig {
    let local_core = Section::new(
        "local.core".to_string(),
        "Local behavior".to_string(),
        None,
        vec![
            Setting::new(
                "local.mode".to_string(),
                "localModeEnabled".to_string(),
                "Local / offline mode".to_string(),
                "Work fully offline on this device only.".to_string(),
                "off".to_string(),
                SettingInputType::Switch,
            ),
            Setting::new(
                "local.encryption".to_string(),
                "encryptionEnabled".to_string(),
                "Encrypt local data".to_string(),
                "Encrypt your notes and settings on this device.".to_string(),
                "on".to_string(),
                SettingInputType::Switch,
            ),
            Setting::new(
                "local.sync".to_string(),
                "syncEnabled".to_string(),
                "Sync in local mode".to_string(),
                "Enable background sync when online mode is active.".to_string(),
                "off".to_string(),
                SettingInputType::Switch,
            ),
            Setting::new(
                "local.showLogs".to_string(),
                "showAppLogs".to_string(),
                "Show application logs".to_string(),
                "Open a view with recent application logs.".to_string(),
                "idle".to_string(),
                SettingInputType::Button,
            ),
            Setting::new(
                "local.exportNotes".to_string(),
                "exportNotes".to_string(),
                "Export notes".to_string(),
                "Export all notes to a backup file on this device.".to_string(),
                "idle".to_string(),
                SettingInputType::Button,
            ),
            Setting::new(
                "local.importNotes".to_string(),
                "importNotes".to_string(),
                "Import notes".to_string(),
                "Import notes from a local backup file.".to_string(),
                "idle".to_string(),
                SettingInputType::Button,
            ),
            Setting::new(
                "local.autoBackup".to_string(),
                "autoBackupEnabled".to_string(),
                "Automatic backups".to_string(),
                "Create automatic backups of your notes.".to_string(),
                "on".to_string(),
                SettingInputType::Switch,
            ),
            Setting::new(
                "local.backupFrequency".to_string(),
                "backupFrequency".to_string(),
                "Backup frequency".to_string(),
                "How often automatic backups are created.".to_string(),
                "daily".to_string(),
                SettingInputType::Select,
            ),
            Setting::new(
                "local.dataDirectory".to_string(),
                "dataDirectory".to_string(),
                "Data directory".to_string(),
                "Location of your local database and files.".to_string(),
                default_data_dir.to_string(),
                SettingInputType::Text,
            ),
        ],
    );
    let local_danger = Section::new(
        "local.danger".to_string(),
        "Danger zone".to_string(),
        None,
        vec![
            Setting::new(
                "local.logout".to_string(),
                "logout".to_string(),
                "Log out".to_string(),
                "Sign out from this account on this device.".to_string(),
                "idle".to_string(),
                SettingInputType::Button,
            ),
            Setting::new(
                "local.deleteLocalFiles".to_string(),
                "deleteLocalFiles".to_string(),
                "Delete local files".to_string(),
                "Permanently delete all local notes and settings on this device.".to_string(),
                "idle".to_string(),
                SettingInputType::Button,
            ),
            Setting::new(
                "local.deleteAccount".to_string(),
                "deleteAccount".to_string(),
                "Delete account".to_string(),
                "Permanently delete your account and all synced data.".to_string(),
                "idle".to_string(),
                SettingInputType::Button,
            ),
        ],
    );

    let online_core = Section::new(
        "online.core".to_string(),
        "Online & sync".to_string(),
        None,
        vec![
            Setting::new(
                "online.mode".to_string(),
                "onlineModeEnabled".to_string(),
                "Online mode".to_string(),
                "Connect this device to your online account.".to_string(),
                "off".to_string(),
                SettingInputType::Switch,
            ),
            Setting::new(
                "online.sync".to_string(),
                "onlineSyncEnabled".to_string(),
                "Sync notes across devices".to_string(),
                "Automatically sync notes across all connected devices.".to_string(),
                "on".to_string(),
                SettingInputType::Switch,
            ),
            Setting::new(
                "online.connectedDevices".to_string(),
                "connectedDevices".to_string(),
                "Connected devices".to_string(),
                "View and manage devices connected to your account.".to_string(),
                "0".to_string(),
                SettingInputType::Info,
            ),
            Setting::new(
                "online.changePasswordEmail".to_string(),
                "changePasswordEmail".to_string(),
                "Change password via email".to_string(),
                "Send a password change link to your email address.".to_string(),
                "idle".to_string(),
                SettingInputType::Button,
            ),
            Setting::new(
                "online.changeUsername".to_string(),
                "changeUsernameOnline".to_string(),
                "Change username".to_string(),
                "Update your online account username.".to_string(),
                "idle".to_string(),
                SettingInputType::Button,
            ),
        ],
    );

    let online_ai = Section::new(
        "online.ai".to_string(),
        "AI & automation".to_string(),
        None,
        vec![Setting::new(
            "online.aiSummary".to_string(),
            "aiSummaryEnabled".to_string(),
            "AI summaries".to_string(),
            "Generate AI summaries for your notes.".to_string(),
            "on".to_string(),
            SettingInputType::Switch,
        )],
    );

    UserConfig {
        sections: vec![
            Section::new(
                "local".to_string(),
                "Local mode settings".to_string(),
                Some(vec![local_core, local_danger]),
                vec![],
            ),
            Section::new(
                "online".to_string(),
                "Online mode settings".to_string(),
                Some(vec![online_core, online_ai]),
                vec![],
            ),
        ],
    }
}

pub struct SettingMeta {
    pub field: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub input_type: SettingInputType,
}

use phf_macros::phf_map;
pub static SETTINGS_META: phf::Map<&'static str, SettingMeta> = phf_map! {

    "local.mode" => SettingMeta {
        field: "localModeEnabled",
        label: "Local / offline mode",
        description: "Work fully offline on this device only.",
        input_type: SettingInputType::Switch,
    },

    "local.encryption" => SettingMeta {
        field: "encryptionEnabled",
        label: "Encrypt local data",
        description: "Encrypt your notes and settings on this device.",
        input_type: SettingInputType::Switch,
    },

    "local.sync" => SettingMeta {
        field: "syncEnabled",
        label: "Sync in local mode",
        description: "Enable background sync when online mode is active.",
        input_type: SettingInputType::Switch,
    },

    "local.showLogs" => SettingMeta {
        field: "showAppLogs",
        label: "Show application logs",
        description: "Open a view with recent application logs.",
        input_type: SettingInputType::Button,
    },

    "local.exportNotes" => SettingMeta {
        field: "exportNotes",
        label: "Export notes",
        description: "Export all notes to a backup file on this device.",
        input_type: SettingInputType::Button,
    },

    "local.importNotes" => SettingMeta {
        field: "importNotes",
        label: "Import notes",
        description: "Import notes from a local backup file.",
        input_type: SettingInputType::Button,
    },

    "local.autoBackup" => SettingMeta {
        field: "autoBackupEnabled",
        label: "Automatic backups",
        description: "Create automatic backups of your notes.",
        input_type: SettingInputType::Switch,
    },

    "local.backupFrequency" => SettingMeta {
        field: "backupFrequency",
        label: "Backup frequency",
        description: "How often automatic backups are created.",
        input_type: SettingInputType::Select,
    },

    "local.dataDirectory" => SettingMeta {
        field: "dataDirectory",
        label: "Data directory",
        description: "Location of your local database and files.",
        input_type: SettingInputType::Text,
    },

    "local.logout" => SettingMeta {
        field: "logout",
        label: "Log out",
        description: "Sign out from this account on this device.",
        input_type: SettingInputType::Button,
    },

    "local.deleteLocalFiles" => SettingMeta {
        field: "deleteLocalFiles",
        label: "Delete local files",
        description: "Permanently delete all local notes and settings on this device.",
        input_type: SettingInputType::Button,
    },

    "local.deleteAccount" => SettingMeta {
        field: "deleteAccount",
        label: "Delete account",
        description: "Permanently delete your account and all synced data.",
        input_type: SettingInputType::Button,
    },

    "online.mode" => SettingMeta {
        field: "onlineModeEnabled",
        label: "Online mode",
        description: "Connect this device to your online account.",
        input_type: SettingInputType::Switch,
    },

    "online.sync" => SettingMeta {
        field: "onlineSyncEnabled",
        label: "Sync notes across devices",
        description: "Automatically sync notes across all connected devices.",
        input_type: SettingInputType::Switch,
    },

    "online.connectedDevices" => SettingMeta {
        field: "connectedDevices",
        label: "Connected devices",
        description: "View and manage devices connected to your account.",
        input_type: SettingInputType::Info,
    },

    "online.changePasswordEmail" => SettingMeta {
        field: "changePasswordEmail",
        label: "Change password via email",
        description: "Send a password change link to your email address.",
        input_type: SettingInputType::Button,
    },

    "online.changeUsername" => SettingMeta {
        field: "changeUsernameOnline",
        label: "Change username",
        description: "Update your online account username.",
        input_type: SettingInputType::Button,
    },

    "online.aiSummary" => SettingMeta {
        field: "aiSummaryEnabled",
        label: "AI summaries",
        description: "Generate AI summaries for your notes.",
        input_type: SettingInputType::Switch,
    },

};
pub struct SectionMeta {
    pub label: &'static str,
}

pub static SECTIONS_META: phf::Map<&'static str, SectionMeta> = phf_map! {

    "local" => SectionMeta {
        label: "Local mode settings",
    },

    "local.core" => SectionMeta {
        label: "Local behavior",
    },

    "local.danger" => SectionMeta {
        label: "Danger zone",
    },

    "online" => SectionMeta {
        label: "Online mode settings",
    },

    "online.core" => SectionMeta {
        label: "Online & sync",
    },

    "online.ai" => SectionMeta {
        label: "AI & automation",
    },

};
