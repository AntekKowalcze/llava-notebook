///Online user struct for mongo
#[allow(dead_code)]
pub struct OnlineUser {
    email: String,
    local_account_info: Vec<(String, uuid::Uuid)>, //username, device id
    password: String,
    public_key: String,
    created_at: i64,
    last_login: i64,
}
// todo when adding online but to consider earlier, is s3 data needed here?
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
//TODO read this and implement data models for users, then create mongo collections. start implementing
// ## PIN vs password — you're right

// Completely agree. PIN is a UX convenience that trades security for speed. If cracking local auth = getting plaintext notes, then local auth needs to be as strong as the content warrants. Password is the right call. The architecture stays identical — you just derive the local key from a password via Argon2id instead of a PIN.

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
