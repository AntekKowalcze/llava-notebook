//! module for database operations
use crate::Note;
use dotenv::dotenv;
use mongodb::{Client, Collection, results::InsertOneResult};
use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;
/// connecting to database with 2 seconds timeout
pub async fn connecting_to_db() -> Result<Collection<Note>, crate::errors::ConnectionError> {
    //getting env file ready
    dotenv().ok();
    //reading uri from env file
    let uri = env::var("MONGODB_URI").expect("uri should be accessible");
    //connecting with timeout
    let client = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        Client::with_uri_str(&uri),
    )
    .await??;

    let db = client.database("notebook");
    Ok(db.collection::<Note>("notebook"))
}
/// function which insert note into database
///
/// # Arguments
///
/// * `my_coll` - reference to connected Collection
///
///  * `inserting_object` - Note
pub async fn inserting_note(
    my_coll: &Collection<Note>,
    mut inserting_object: Note,
) -> Result<InsertOneResult, mongodb::error::Error> {
    let insert_note: InsertOneResult = my_coll.insert_one(&inserting_object).await?;
    let note_id = insert_note
        .inserted_id
        .as_object_id()
        .expect("couldnt convert object id");
    inserting_object.note_id = Some(note_id);
    println!("note_with_id: {inserting_object :?}");
    Ok(insert_note)
}

pub async fn inserting_from_vec_after_reconnection(
    local_vec: Arc<RwLock<Vec<Note>>>,
    my_coll: &Collection<Note>,
) {
    println!("inserting notes to database");
    let mut write_vec = local_vec.write().await;
    if write_vec.len() > 0 {
        while let Some(note) = write_vec.pop() {
            match inserting_note(my_coll, note).await {
                Ok(id) => println!("inserted {id:?}"),
                Err(_) => println!("couldn’t insert note"),
            }
        }
        println!("vec: {write_vec :?}");
        // let mut i = 0;
        // while i < write_vec.len() {
        //     if let Ok(id) = inserting_note(my_coll, &write_vec[i]).await {
        //         println!("inserted {id :?}");
        //         let _v = write_vec.remove(i);
        //     } else {
        //         println!("couldnt add note");
        //         i += 1;
        //     }
        //     println!("vec: {write_vec :?}")
        // }
    } else {
        println!("notes are up to date");
    }
}
