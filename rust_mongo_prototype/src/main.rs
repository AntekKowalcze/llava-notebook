mod db;
mod errors;
mod helpers;
mod models;
mod save_locally;
mod test;
use crate::db::inserting_from_vec_after_reconnection;
use crate::models::Note;
use crate::save_locally::save_locally;
use chrono;
use mongodb::Collection;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() {
    let local_note_storage: Arc<RwLock<Vec<Note>>> = Arc::new(RwLock::new(Vec::new()));
    let my_coll: Arc<RwLock<Option<Collection<Note>>>> = Arc::new(RwLock::new(None));
    let coll_clone = my_coll.clone();

    let cloned_note_storage = local_note_storage.clone();
    tokio::spawn(async move {
        loop {
            let is_connected = {
                let read = my_coll.read().await;
                read.is_some()
            };
            if is_connected {
                println!("db connected");
            } else {
                println!("Attempting to connect...");
                let connection = db::connecting_to_db().await;
                let mut write = my_coll.write().await;
                match connection {
                    Ok(coll) => {
                        println!("Connected successfully");
                        let cloned_colll = coll.clone();
                        *write = Some(coll);
                        inserting_from_vec_after_reconnection(
                            local_note_storage.clone(),
                            &cloned_colll,
                        )
                        .await //add
                    }
                    Err(err) => {
                        println!("{err:?}, values will be saved locally until internet connection");
                        *write = None;
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    loop {
        println!("choose option:");
        println!("1. new note\n2. update note\n3. delete note\n4.read note\n5. exit ");
        let mut option = String::new();
        std::io::stdin()
            .read_line(&mut option)
            .expect("couldnt get user input exiting");
        let option: i32 = option
            .trim()
            .parse::<i32>()
            .expect("no option like this, exitting");
        match option {
            1 => {
                let new_note = create_note();
                let cloned_note = new_note.clone();
                let result = crate::helpers::check_connection_and_execute_action(
                    coll_clone.clone(),
                    |c| async move { crate::db::inserting_note(&c, &new_note).await },
                )
                .await;

                match result {
                    Ok(result) => {
                        println!("{result :?}")
                    }
                    Err(error) => {
                        println!("{error}");
                        save_locally(cloned_note, cloned_note_storage.clone()).await
                    }
                }
            }
            2 => {} //read prompt note title to find note check first in locally cuz its faster, then check into database by title and if the same title exist show both summaries and tell to pick, show it
            3 => {} //update gprompt note title to find note check first in locally cuz its faster, then check into database by title and if the same title exist show both summaries and tell to pick, and edit
            4 => {} //prompt note title to find note check first in locally cuz its faster, then check into database by title and if the same title exist show both summaries and tell to pick, show delete it
            //wykonywać to w helperze który sprawdza połączenie w zależności od tego, dać w result error sprawdzanie lokalne i osobną funkcje do wybierania
            // helper, musi brać arc rwlock dla połaczenia, musi prac arc rwlock vector obie rzeczy zklonowane, robić read i odrazu drop, jeśli połączenie sprawdzić i lokalnie w vec i w połączeniu jeśl nie komunikat że bedzie tylko z lokalnych notatek
            //jeśli identyczne tytułu pobrać dla wszystich summary i wyświetlić z numerami, przypiasć numery do summary żeby się to jakoś dało odróżnić od siebie
            // musi robić deserializacje do structa lub jakoś inaczej to wyświetlać (sprawdzić)
            5 => {
                let local_note_to_read = { cloned_note_storage.read().await.clone() };
                println!("{local_note_to_read :?}");
                std::process::exit(1)
            }
            _ => panic!("no option like this exitting",),
        }
    }
} //po polączeniu dodać wszystkie notatki z vectora
///creating note by getting values from other functions
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
        created_at: created_at,
        title: title,
        summary: summary_string,
        content: content,
    }
}
///getting date and time
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
