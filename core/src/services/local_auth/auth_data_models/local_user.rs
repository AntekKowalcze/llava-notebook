///data model of local user
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct LocalUser {
    #[zeroize(skip)]
    pub user_id: uuid::Uuid,
    pub username: String,
    pub password_hash: String,
    // LOCAL ENCRYPTION
    pub notes_key: Vec<u8>,
    pub nonce_notes_key: Vec<u8>,
    pub kek_salt: String,
    pub kek_argon_params: String,

    pub is_online_linked: bool,
    pub online_account_email: Option<String>,
    pub online_account_id: Option<String>,
    #[zeroize(skip)]
    pub device_id: uuid::Uuid,
    pub created_at: i64,
    pub last_login: i64,
    pub password_errors: i64,
    pub ending_block_timestamp: i64,
}
