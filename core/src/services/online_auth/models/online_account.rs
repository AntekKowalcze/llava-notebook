/// Online authentication models shared across client/server boundaries.
use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};
use uuid::Uuid;
#[derive(Debug, Serialize, Deserialize)]
pub struct ArgonParams {
    pub m_cost: u32,
    pub t_cost: u32,
    pub p_cost: u32,
}

pub struct RegisterRequest {
    user: RegisterUserPayload,
    device: RegisterDevicePayload,
}

/// Sent to the server on registration — only what client owns
#[serde_as]
#[derive(Debug, Serialize)]
pub struct RegisterUserPayload {
    pub email: String,
    pub password_hash: String,
    #[serde_as(as = "Base64")]
    pub master_key_enc: Vec<u8>,
    #[serde_as(as = "Base64")]
    pub master_key_nonce: Vec<u8>,
    pub kek_salt: String,
    pub argon2_params: ArgonParams,
}

/// Received from the server — full user object
#[serde_as]
#[derive(Debug, Deserialize)]
pub struct OnlineUser {
    #[serde(rename = "_id")]
    pub id: Option<String>, // server returns ObjectId as string
    pub email: String,
    pub email_verified: bool,
    #[serde_as(as = "Base64")]
    pub master_key_enc: Vec<u8>,
    #[serde_as(as = "Base64")]
    pub master_key_nonce: Vec<u8>,
    pub kek_salt: String,
    pub argon2_params: ArgonParams,
    pub storage_used_bytes: i64,
    pub quota_bytes: i64,
    pub failed_attempts: i64,
    pub lockout_until: Option<i64>,
    pub created_at: i64,
    pub last_login: i64,
}
/// Decoded contents of an access token JWT
#[derive(Debug, Serialize, Deserialize)]
pub struct AccessTokenPayload {
    pub sub: String, // user ObjectId as string
    pub device_id: Uuid,
    pub exp: i64,
    pub iat: i64,
}

/// Raw access token — just a wrapper so you don't pass bare strings around
pub struct AccessToken(pub String);

/// Sent to server when linking/registering a device
#[derive(Debug, Serialize)]
pub struct RegisterDevicePayload {
    pub device_name: String,
    pub device_id: Uuid,
}

/// Received from server — full device object
#[derive(Debug, Deserialize)]
pub struct Device {
    #[serde(rename = "_id")]
    pub id: Option<String>,
    pub device_id: Uuid,
    pub user_id: String,
    pub device_name: String,
    pub last_seen: i64,
    pub created_at: i64,
}

/// Stored in OS keyring, sent to server on refresh
#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshToken {
    pub jti: Uuid,
    pub expires_at: i64,
}

/// Server response after login or token refresh
#[derive(Debug, Deserialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub device_id: Uuid,
}

// {
//   "_id": "uuid",
//   "email": "string",
//   "email_verified": "bool",
//   "password_hash": "argon2id hash",
//   "master_key_enc": "base64",
//   "master_key_nonce": "base64",
//   "kek_salt": "base64",
//   "argon2_params": { "m_cost": 65536, "t_cost": 3, "p_cost": 4 },
//   "storage_used_bytes": 0,
//   "quota_bytes": 1073741824,
//   "failed_attempts": 0,
//   "lockout_until": null,
//   "created_at": "timestamp",
//   "last_login": "timestamp"
// }
//mongo ONLINE
// mongo id
// email
// localusernames [
//{ username
// device_id
//}
//]
// password
//public_key
// created_at
// last login
//change_pin

//okay my flow looks like this user logs in locally, and gets popup for passing pin/password for online account (in overall sync between devices)

// ## Online user model

// Your list is correct but devices should be a **separate collection** — one user will have multiple devices:
// ```
// users collection:
// {
//   _id: uuid,
//   email: string,
//   email_verified: bool,          // you'll want this
//   password_hash: string,         // argon2id

//   master_key_enc: string,        // base64 ciphertext
//   master_key_nonce: string,      // base64
//   kek_salt: string,              // base64
//   argon2_params: {               // store params so you can upgrade later
//     m_cost, t_cost, p_cost
//   },
//   refresh token: string          //access token is stored in keyring, refresh in mongo
//   storage_used_bytes: number,    // you'll need this for quota enforcement
//   quota_bytes: number,

//   failed_attempts: number,       // for lockout
//   lockout_until: timestamp,      // your "ending block timestamp"

//   created_at: timestamp,
//   last_login: timestamp
// }

// devices collection:
// {
//   _id: uuid,
//   user_id: uuid,                 // FK to users
//   device_name: string,           // "Laptop", "Phone"
//   last_seen: timestamp,
//   created_at: timestamp
// }

//JWT data email, id, device id

//HOW JWT WORKS IN THIS APP

// go server is a middlleware between client and mongo db, its tasks it to check wheather user can do operation (authorization) wheather user is who he si (authentication) - JWT and then it should have sync logic, which is server time based, so there is no time zone miss understanding etc, all mongo queries happen on server
// client do all hevy job encrypting and preparing data, server just do crud operations.
