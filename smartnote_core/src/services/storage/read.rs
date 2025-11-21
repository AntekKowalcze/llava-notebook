use std::fs;
///function responsible for reading notes content
fn read_note_content(
    paths: &crate::config::ProgramFiles,
    name: String,
) -> Result<String, crate::errors::Error> {
    let display_content = fs::read_to_string(paths.notes_path.join(format!("{name}.md")))?;
    Ok(display_content)
}

#[test]
fn read_test() {
    let paths = crate::config::ProgramFiles::init().unwrap();
    let name = "this_is_tets_note".to_string();
    let content = read_note_content(&paths, name).unwrap();
    println!("{content}")
}
