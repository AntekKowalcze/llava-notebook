mod db;
mod models;
mod save_locally;
use crate::db::inserting_note;
use crate::models::Note;
use crate::save_locally::save_locally;
use chrono;

//na początku połączyć się z bazą danych tryb offline
#[tokio::main]
async fn main() {
    let my_coll = crate::db::connecting_to_db().await; //connecting to database
    let mut local_note_storage: Vec<Note> = Vec::new(); // creatign local storage 
    let mut new_note = create_note(); //creating note
    match my_coll {
        Ok(my_coll) => match inserting_note(my_coll, &mut new_note).await {
            //if connected successfully insert else try to reconnect
            Ok(inserted_id) => {
                // insert or try to reconnect and reinsert
                let result = inserted_id
                    .inserted_id
                    .as_object_id()
                    .expect("InsertOneResult conversion faild in recconect and add note fn");
                println!("added note id: {}", result);
            }
            Err(err) => {
                eprintln!("{err}");
                crate::db::reconnect_and_add_note(&mut local_note_storage, new_note).await;
            }
        },
        Err(err) => {
            eprintln!("{err}");
            crate::save_locally(new_note, &mut local_note_storage);
        }
    }
}

fn create_note() -> Note {
    let created_at = getting_created_at();
    let title = String::from(get_title().trim());
    let content = String::from(get_content().trim());
    let summary: Vec<&str> = content.split(" ").collect();
    let mut summary_string = String::from("");
    if summary.len() < 10 {
        summary_string = content.clone(); //copy is not big cuz its less than 10 words (in future check lenght to avoid huge charstrings to be copied)
    } else {
        for i in 0..9 {
            summary_string = format!("{} {}", summary_string, summary[i]);
        }
    }
    Note {
        note_id: None,
        created_at: created_at,
        title: title,
        summary: summary_string,
        content: content,
    }
}

fn getting_created_at() -> String {
    println!("{:?}", chrono::offset::Utc::now().time());
    let day = chrono::offset::Utc::now().date_naive().to_string();
    let time = &chrono::offset::Utc::now().time().to_string()[0..8]; //getting time to seconds
    let created_at = format!("{} {}", day, time);
    created_at
}

fn get_title() -> String {
    let mut title = String::new();
    loop {
        println!("Insert title here: ");
        match std::io::stdin().read_line(&mut title) {
            Ok(_) => break,
            Err(error) => {
                println!("{error}");
                continue;
            }
        }
    }
    title
}

fn get_content() -> String {
    let mut content = String::new();
    loop {
        println!("Insert content here: ");
        match std::io::stdin().read_line(&mut content) {
            Ok(_) => break,
            Err(error) => {
                println!("{error}");
                continue;
            }
        }
    } //in the future make it leavable, to leave creation by typing smth, also in title
    content
}
