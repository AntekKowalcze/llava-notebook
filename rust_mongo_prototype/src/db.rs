use dotenv::dotenv;
use mongodb::{Client, Collection, results::InsertOneResult};
use std::env;

use crate::Note;

pub async fn connecting_to_db() -> mongodb::error::Result<Collection<Note>> {
    //connecting to database
    dotenv().ok();
    let uri = env::var("MONGODB_URI").expect("uri should be accessible");
    let client = Client::with_uri_str(uri).await?;
    let database = client.database("notebook");
    let my_coll: Collection<Note> = database.collection("notebook");
    Ok(my_coll)
}

pub async fn inserting_note(
    my_coll: Collection<Note>,
    inserting_object: &Note,
) -> Result<InsertOneResult, mongodb::error::Error> {
    let insert_note: InsertOneResult = my_coll.insert_one(inserting_object).await?;
    Ok(insert_note)
}
///try recconecting to database 3 times if its not possible
/// try readding to database 3 times if failed
pub async fn reconnect_and_add_note(local_note_storage: &mut Vec<Note>, inserting_object: Note) {
    for i in 0..3 {
        let my_coll = crate::db::connecting_to_db().await;
        match my_coll {
            // online saving
            Ok(my_coll) => match crate::db::inserting_note(my_coll, &inserting_object).await {
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
                    println!("failed to connect {}, {err}", i + 1);
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    continue;
                } else {
                    println!("failed to connect {}, {err}", i + 1);
                    crate::save_locally(inserting_object, local_note_storage);

                    break;
                }
            }
        };
    }
}
