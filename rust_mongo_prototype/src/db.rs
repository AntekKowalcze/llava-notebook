//! module for database operations
use dotenv::dotenv;
use mongodb::{Client, Collection, results::InsertOneResult};
use std::env;

use crate::Note;
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

pub async fn inserting_note(
    my_coll: Collection<Note>,
    inserting_object: &mut Note,
) -> Result<InsertOneResult, mongodb::error::Error> {
    let last_note_id = my_coll
        .estimated_document_count()
        .await
        .map(|id| id + 1)
        .ok();
    inserting_object.note_id = last_note_id;
    let insert_note: InsertOneResult = my_coll.insert_one(inserting_object).await?;
    Ok(insert_note)
}
///try recconecting to database 3 times if its not possible
/// try readding to database 3 times if failed
pub async fn reconnect_and_add_note(
    local_note_storage: &mut Vec<Note>,
    mut inserting_object: Note,
) {
    for i in 0..3 {
        let my_coll = crate::db::connecting_to_db().await;
        match my_coll {
            // online saving
            Ok(my_coll) => match crate::db::inserting_note(my_coll, &mut inserting_object).await {
                Ok(inserted_id) => {
                    let result = inserted_id
                        .inserted_id
                        .as_object_id()
                        .expect("InsertOneResult conversion faild in recconect and add note fn");
                    println!("added note id: {}", result);
                    break;
                }
                Err(_) => {
                    if i < 3 {
                        println!("failed insert {}", i + 1);
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await; //sleep to avoid 3 rapid tryes
                        continue;
                    } else {
                        crate::save_locally(inserting_object, local_note_storage);

                        break;
                    }
                }
            },
            //offline saving
            Err(err) => {
                if i < 3 {
                    println!("failed to connect {}, {err :?}", i + 1);
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    continue;
                } else {
                    println!("failed to connect {}, {err :?}", i + 1);
                    crate::save_locally(inserting_object, local_note_storage);
                    break;
                }
            }
        };
    }
}
