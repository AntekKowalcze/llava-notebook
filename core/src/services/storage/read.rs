use std::fs;
///function responsible for reading notes content
pub fn read_note_content(
    //możę zmienic z name na uuid i path wywalić do kosza i przepisać na query po id zeby zdobyć path
    paths: &crate::config::ProgramFiles,
    name: String,
) -> Result<String, crate::errors::Error> {
    let display_content = fs::read_to_string(paths.notes_path.join(format!("{name}.md")))?;
    Ok(display_content)
}

#[test]
fn read_test() {
    let paths = crate::config::ProgramFiles::init_in_base().unwrap();
    let name = "tttsss".to_string();
    let content = read_note_content(&paths, name).unwrap();
}
