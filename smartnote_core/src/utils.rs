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
