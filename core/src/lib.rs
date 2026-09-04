// lib.rs

mod config;
mod constants;
mod crypto;
mod errors;
mod migrations;
mod models;
mod services;
mod tags;
mod utils;

pub mod local_auth {
    pub use crate::services::local_auth::database_creation::connect_or_create_local_login_db;
    pub use crate::services::local_auth::local_login::SessionState;
    pub use crate::services::local_auth::local_login::{
        autorization, change_last_login, check_error_count, check_if_user_logged_in,
        check_online_login, get_optional_online_id, get_timeout, invalidate_recovery_keys,
        invalidate_session_for_user, local_log_in, local_logout, log_with_code, session_operations,
        zero_error_count,
    };
    pub use crate::services::local_auth::register::{
        change_password, generate_recovery_codes_with_new_key, recovery_code_handling,
        register_user_offline, rewrap_key,
    };
    pub use crate::services::local_auth::utils::check_if_first_start;
}

pub mod storage {
    pub use crate::services::storage::db_creation::{SyncState, get_connection};
    pub use crate::services::storage::delete::delete_note;
    pub use crate::services::storage::init_note::add_note_to_database;
    pub use crate::services::storage::init_note::create_local_note;
    pub use crate::services::storage::note_operations::get_note_struct;
    pub use crate::services::storage::note_operations::get_title;
    pub use crate::services::storage::note_operations::hard_delete_note;
    pub use crate::services::storage::note_operations::remove_note;
    pub use crate::services::storage::note_operations::restore_deleted_note;
    pub use crate::services::storage::note_operations::update_title;
    pub use crate::services::storage::note_operations::{
        check_if_note_is_encrypted, check_if_note_is_synced, get_note, get_note_content,
        toggle_note_encryption, toggle_note_sync, verify_note_owner, resolve_attachment_protocol, change_sync_to_pending_upload
    };
    pub use crate::services::storage::update::update_md;
}
pub mod tags_handling {
    pub use crate::tags::UiTag;
    pub use crate::tags::add_tag_to_database;
    pub use crate::tags::add_tag_to_note;
    pub use crate::tags::find_tag_id;
    pub use crate::tags::get_all_tags;
    pub use crate::tags::get_all_tags_for_note;
    pub use crate::tags::remove_tag;
    pub use crate::tags::remove_tag_from_note;
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
    pub use crate::services::user_stats::sliding_panel::{PanelData, get_sliding_panel_stats};
}

pub mod online_auth {
    pub use crate::models::online_account::AccessToken;
    pub use crate::services::online_auth::login::check_if_logged_in_online;
    pub use crate::services::online_auth::login::login;
    pub use crate::services::online_auth::logout::delete_synced_notes_on_logout;
    pub use crate::services::online_auth::logout::logout;
    pub use crate::services::online_auth::logout::set_account_to_offline_in_db;
    pub use crate::services::online_auth::register::change_email_in_database;
    pub use crate::services::online_auth::register::register;
}

pub mod crypto_operations {
    pub use crate::crypto::decrypt_attachment;
    pub use crate::crypto::decrypt_note;
    pub use crate::crypto::decrypt_title;
    pub use crate::crypto::encrypt_attachment;
    pub use crate::crypto::encrypt_data;
    pub use crate::crypto::encrypt_title;
    pub use crate::crypto::reencrypt_db;
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

pub mod note_stats {
    pub use crate::services::storage::note_managing::NoteCard;
    pub use crate::services::storage::note_managing::RemovedNote;
    pub use crate::services::storage::note_managing::get_all_notes_data;
    pub use crate::services::storage::note_managing::get_all_removed_notes_data;
}

pub mod clean {
    pub use crate::services::cleaner::hard_deletes_terminated_notes;
}
pub mod attachments {
    pub use crate::models::attachment::Attachment;
    pub use crate::services::attachment::check_attachment_existance;
    pub use crate::services::attachment::check_if_attachment_is_encrypted;
    pub use crate::services::attachment::create_attachment;
    pub use crate::services::attachment::delete_attachment;
    pub use crate::services::attachment::get_attachments_for_note;
    pub use crate::services::attachment::read_attachment;
    pub use crate::services::attachment::toggle_attachments_encryption_for_note;
    pub use crate::services::attachment::toggle_attachments_sync_for_note;
    pub use crate::services::attachment::update_attachment_file;
}

pub mod sync {
    pub use crate::models::sync::AttachmentForUpload;
    pub use crate::models::sync::CheckNoteSyncStatus;
    pub use crate::models::sync::CheckSyncResponse;
    pub use crate::models::sync::NoteForUpload;
    pub use crate::models::sync::UploadAttachment;
    pub use crate::services::sync::DbOperation;
    pub use crate::services::sync::download_attachment;
    pub use crate::services::sync::execute_db_operations;
    pub use crate::services::sync::execute_server_operations;
    pub use crate::services::sync::get_all_notes_to_sync;
    pub use crate::services::sync::get_attachment_for_upload;
    pub use crate::services::sync::get_note_for_upload;
    pub use crate::services::sync::handle_attachment_synced;
    pub use crate::services::sync::handle_attachments_to_hard_delete;
    pub use crate::services::sync::handle_notes_synced;
    pub use crate::services::sync::handle_notes_to_download;
    pub use crate::services::sync::handle_notes_to_hard_delete;
    pub use crate::services::sync::sync;
    pub use crate::services::sync::upload_attachments;
    pub use crate::services::sync::upload_notes;
}

pub mod ai {
    pub use crate::services::ai::AiPromptContext;
    pub use crate::services::ai::send_ai_request;
}
