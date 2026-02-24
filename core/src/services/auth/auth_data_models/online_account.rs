///Online user struct for mongo
#[allow(dead_code)]
pub struct OnlineUser {
    email: String,
    localusernames: Vec<(String, uuid::Uuid)>, //username, device id
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
