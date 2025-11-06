pub fn getting_user_input(buffer: &mut String) {
    println!("Podaj treść");
    std::io::stdin()
        .read_line(buffer)
        .expect("getting input failed");
    //im using expect cuz it wont be used in application only for testing
}
