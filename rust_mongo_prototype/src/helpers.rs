// //! module with helper functions
use crate::models::Note;
use dialoguer::Input;
use futures::stream::TryStreamExt;
use mongodb::Collection;
use std::sync::Arc;
use tokio::sync::RwLock;
///helpler which checks connection and execute function
/// # Arguments
///
/// * `coll` - shared reference to option collection
/// *  `f` - closure with function to execute
///
/// # Returns
///
/// Returns Result, Ok() - with value of Ok value of function, and
pub async fn check_connection_and_execute_action<F, Fut, T>(
    coll: Arc<RwLock<Option<Collection<crate::models::Note>>>>,
    f: F,
) -> Result<T, String>
where
    F: FnOnce(Collection<crate::models::Note>) -> Fut + Send,
    Fut: std::future::Future<Output = Result<T, mongodb::error::Error>> + Send,
{
    let maybe_cloned = {
        let maybe_coll = coll.read().await;
        maybe_coll.clone() // Clone Option<Collection>
    };

    if let Some(my_coll) = maybe_cloned {
        match f(my_coll).await {
            Ok(result) => Ok(result),
            Err(_) => {
                println!("Lost connection");
                *coll.write().await = None;
                Err("Lost connection".to_string())
            }
        }
    } else {
        Err("No connection".to_string())
    }
}
//dalej dodać aby wyświetlał z tego vectora found tytuł oraz summary i dawał opcje wyboru po indexie + 1 vectora, i tą właśnie notatke przekazywa… do closure z funckją która ma być wykonywana, tyle że
//przy update i remove trzeba znaleźć identyczną notatke w db przez find i działać na niej/

//dodac że jeśli nie znalazło, vector jest pusty to daje błąd
pub async fn find_note_by_title(
    coll: Arc<RwLock<Option<Collection<Note>>>>,
    note_vec: Arc<RwLock<Vec<crate::models::Note>>>,
) -> Result<Option<Note>, mongodb::error::Error> {
    let note_title = get_title_name();
    let mut found_notes: Vec<Note> = Vec::new();
    let vec_read = note_vec.read().await;
    for note in vec_read.iter() {
        if note.title == note_title {
            found_notes.push(note.clone());
        }
    }
    let coll_read = {
        let coll_rw = coll.read().await;
        coll_rw.clone()
    };

    match coll_read {
        Some(my_coll) => {
            let filter = mongodb::bson::doc! {"title": &note_title};
            let mut coursor = my_coll.find(filter).await?;
            while let Some(note) = coursor.try_next().await? {
                println!("adding to found vec from db");

                found_notes.push(note);
            }
        }
        None => {
            println!("no internet connection, finding only in notes saved locally")
        }
    }
    if found_notes.is_empty() {
        eprintln!("There are no notes with this title");
        return Ok(None);
    } else {
        let found_notes_count = found_notes.len();
        println!("Found {found_notes_count}\nchoose note:");

        let mut index = 1;
        for note in &found_notes {
            println!("{index}. {}", note.title);
            println!("{}\n\n", note.summary);
            index += 1;
        }
        let mut note_choosen = String::new();
        std::io::stdin()
            .read_line(&mut note_choosen)
            .expect("there is no note like this found");

        let note_choosen = note_choosen
            .trim()
            .parse::<usize>()
            .expect("input wasn't an number");
        let note_found = found_notes.remove(note_choosen - 1);
        Ok(Some(note_found))
    }
    // println!("found notes: {found_notes :?}"); //to i ok do wywalenia, prawdopodobnie bedzie zwracać  Option na znalezioną notatkę albo nic lub błąd
    // Ok(found_notes)
}

fn get_title_name() -> String {
    println!("Type note title:");
    let mut note_title = String::new();
    std::io::stdin()
        .read_line(&mut note_title)
        .expect("input cant be red");
    note_title.trim().to_string()
}

pub async fn read_delete_update<F, Fut>(
    coll: std::sync::Arc<tokio::sync::RwLock<Option<mongodb::Collection<crate::models::Note>>>>,
    note_vec: std::sync::Arc<tokio::sync::RwLock<Vec<crate::models::Note>>>,
    f: F,
) where
    F: FnOnce(crate::models::Note, Option<Arc<RwLock<Option<Collection<Note>>>>>) -> Fut,
    Fut: Future<Output = ()>,
{
    let arc_coll = coll.clone();
    let note_r = crate::helpers::find_note_by_title(coll, note_vec).await;
    if let Ok(note_o) = note_r {
        if let Some(note) = note_o {
            f(note, Some(arc_coll.clone())).await;
        } else {
            println!("There is no note with this name")
        }
    } else {
        let mut write_c = arc_coll.write().await;
        *write_c = None;
        println!("Lost connection");
    }
}

pub fn get_note_field() -> i64 {
    println!("Choose a field to update");
    println!("1. Title\n2. Summary\n3. Content\n ");
    let mut field = String::new();
    std::io::stdin()
        .read_line(&mut field)
        .expect("no field like this");
    let field = field
        .trim()
        .parse::<i64>()
        .expect("couldnt parse intpu, expected a number from 1 to 3");
    field
}

pub fn edit_note(field_value: String) -> anyhow::Result<String> {
    let edited: String = Input::new()
        .with_prompt("Edit field")
        .with_initial_text(field_value.clone()) // pre-filled text the user can edit
        .default(field_value) // if user submits empty, keep original
        .interact_text()?; // reads edited content on Enter 
    Ok(edited)
}
