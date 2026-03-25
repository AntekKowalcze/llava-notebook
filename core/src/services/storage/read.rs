use std::fs;
///function responsible for reading notes content
pub fn read_note_content(
    //change from name to uuid and throw path to trash, get name from database
    paths: &crate::config::ProgramFiles,
    name: String,
) -> Result<String, crate::errors::Error> {
    let display_content = fs::read_to_string(paths.notes_path.join(format!("{name}.md")))?;
    Ok(display_content)
}

//run when notes are created
// #[test]
// fn read_test() {
//     let paths = crate::config::ProgramFiles::init_in_base().unwrap();
//     let name = "tttsss".to_string();
//     let _content = read_note_content(&paths, name).unwrap();
// }
