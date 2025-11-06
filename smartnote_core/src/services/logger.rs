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
