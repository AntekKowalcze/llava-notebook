///data model of local user
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct LocalUser {
    #[zeroize(skip)]
    pub user_id: uuid::Uuid,
    pub username: String,
    pub password_hash: String, //hashed
    pub password_salt: String,
    pub notes_key: Vec<u8>,
    pub nonce_notes_key: Vec<u8>,
    pub is_online_linked: bool,
    pub online_account_email: Option<String>,
    #[zeroize(skip)]
    pub device_id: uuid::Uuid,
    pub created_at: i64,
    pub last_login: i64,
}

//now sqlite LOCAL db
//  id INT auto increment, primary key
//  username VARCHAR(20)
// password hashed password
// is_online_linked = 1
// online_account_email Option email
//device id //
// created_at INTEGER;
// last_login INTEGER;

//
// 	Value	Why
// journal_mode = WAL <
// synchronous = NORMAL<
// cache_size = -2000<
// temp_store = MEMORY<

// index on user name
//
