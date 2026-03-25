// lib.rs

mod config;
mod constants;
mod errors;
mod models;
mod services;
mod utils;

pub mod auth {
    pub use crate::services::auth::database_creation::connect_or_create_local_login_db;
    pub use crate::services::auth::logging::SessionState;
    pub use crate::services::auth::logging::{
        change_last_login, check_error_count, check_if_user_logged_in, get_timeout, local_log_in,
        local_logout, log_with_code, zero_error_count,
    };
    pub use crate::services::auth::register::{
        change_password, recovery_code_handling, register_user_offline,
    };
    pub use crate::services::auth::utils::check_if_first_start;
}

pub mod storage {
    pub use crate::services::storage::db_creation::{SyncState, get_connection};
    pub use crate::services::storage::delete::delete_note;
    pub use crate::services::storage::init_note::add_note_to_database;
    pub use crate::services::storage::read::read_note_content;
    pub use crate::services::storage::update::update_md;
}

pub mod settings {
    pub use crate::services::user_settings::metaphone::create_metaphone_map;
    pub use crate::services::user_settings::settings::{
        UserConfig, get_config, get_config_for_state, save_config,
    };
}

pub mod stats {
    pub use crate::services::user_stats::dashboard_stats::{DashboardData, get_dashboard_stats};
}

pub use config::get_device_id;
pub use config::get_paths;
pub use config::{AppState, ProgramFiles};
pub use errors::Error;
pub use models::note::Note;
pub use services::logger::configure_logger;
pub use utils::{get_time, get_user_uuid, get_username_from_uuid};
