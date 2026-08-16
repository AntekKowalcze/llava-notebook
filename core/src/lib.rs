// lib.rs

mod config;
mod constants;
mod errors;
mod migrations;
mod models;
mod services;
mod utils;
mod crypto;
mod tags;

pub mod local_auth {
    pub use crate::services::local_auth::database_creation::connect_or_create_local_login_db;
    pub use crate::services::local_auth::logging::SessionState;
    pub use crate::services::local_auth::logging::{
        autorization, change_last_login, check_error_count, check_if_user_logged_in,
        check_online_login, get_optional_online_id, get_timeout, local_log_in, local_logout,
        log_with_code, zero_error_count,
    };
    pub use crate::services::local_auth::register::{
        change_password, recovery_code_handling, register_user_offline,
    };
    pub use crate::services::local_auth::utils::check_if_first_start;
}

pub mod storage {
    pub use crate::services::storage::db_creation::{SyncState, get_connection};
    pub use crate::services::storage::delete::delete_note;
    pub use crate::services::storage::init_note::add_note_to_database;
    pub use crate::services::storage::update::update_md;
    pub use crate::services::storage::init_note::create_local_note;
    pub use crate::services::storage::note_operations::{verify_note_owner, get_note, get_note_content, check_if_note_is_encrypted, toggle_note_encryption, toggle_note_sync};
    pub use crate::services::storage::note_operations::update_title;
    pub use crate::services::storage::note_operations::get_title;
    pub use crate::services::storage::note_operations::get_note_struct;

    
}
pub mod tags_handling {
    pub use crate::tags::add_tag_to_note;
    pub use crate::tags::add_tag_to_database;
    pub use crate::tags::remove_tag;
    pub use crate::tags::remove_tag_from_note;
    pub use crate::tags::UiTag;
    pub use crate::tags::find_tag_id;
    pub use crate::tags::get_all_tags;
    pub use crate::tags::get_all_tags_for_note;
}

pub mod settings {
    pub use crate::services::user_settings::metaphone::create_metaphone_map;
    pub use crate::services::user_settings::setting_actions::{change_username, logfile_contents};
    pub use crate::services::user_settings::settings::load_config_backup;
    pub use crate::services::user_settings::settings::{
        UserConfig, get_config, get_config_for_state, save_config,
    };
}

pub mod stats {
    pub use crate::services::user_stats::dashboard_stats::{DashboardData, get_dashboard_stats};
    pub use crate::services::user_stats::sliding_panel::{PanelData,get_sliding_panel_stats};
}

pub mod online_auth {
    pub use crate::services::online_auth::login::check_if_logged_in_online;
    pub use crate::services::online_auth::login::login;
    pub use crate::services::online_auth::logout::logout;
    pub use crate::services::online_auth::register::change_email_in_database;
    pub use crate::services::online_auth::register::register;
}

pub mod crypto_operations {
    pub use crate::crypto::decrypt_note;
    pub use crate::crypto::encrypt_data;
    pub use crate::crypto::encrypt_title;
    pub use crate::crypto::decrypt_title;
}

pub use config::get_device_id;
pub use config::get_paths;
pub use config::{AppState, ProgramFiles};
pub use errors::Error;
pub use models::note::Note;
pub use services::logger::configure_logger;
pub use utils::{
    change_account_link_status, check_connection, get_email_from_online_id, get_online_id,
    get_time, get_user_uuid, get_username_from_uuid, is_online_linked,
};

