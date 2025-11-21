//! Module for application logger, temporarry holds terminal logging
use std::fmt::Debug;

pub fn log_success(log_content: &str) {
    println!("✅ {}", log_content)
}

pub fn log_error<T>(log_content: &str, error: T)
where
    T: Debug,
{
    eprintln!("❌ {:?}, {}", error, log_content)
}
//TODO dodać opóźnienie po iluś źle wpisanych hasłach
//TODO Brak synchronizacji między FS i DB - w delete_note() robisz fs::rename() PRZED UPDATE, więc jeśli DB update zawiedzie, plik będzie już przeniesiony. Odwróć kolejność lub użyj two-phase commit pattern
