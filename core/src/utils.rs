//! modules for useful tools
pub fn getting_user_input(buffer: &mut String) {
    println!("Podaj treść");
    std::io::stdin()
        .read_line(buffer)
        .expect("getting input failed");
    //im using expect cuz it wont be used in application only for testing
}
///gets time in UTC timestamp i64
pub fn get_time() -> i64 {
    let time = chrono::Utc::now();
    time.timestamp()
}

pub enum Format<'a, T> {
    Display(&'a T),
    Debug(&'a T),
}

pub fn log_helper<T>(task: &str, status: &str, additional_info: Option<Format<T>>, context: &str)
where
    T: std::fmt::Display + std::fmt::Debug,
{
    match additional_info {
        Some(Format::Display(v)) => tracing::info!(task = task, status = status, %v, context),
        Some(Format::Debug(v)) => tracing::info!(task = task, status = status, ?v, context),
        None => tracing::info!(task = task, status = status, context),
    }
}
