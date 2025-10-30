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
/// function which insert note into database
///
/// # Arguments
///
/// * `my_coll` - reference to connected Collection
///
///  * `inserting_object` - Note
pub async fn inserting_note(
    my_coll: &Collection<Note>,
    inserting_object: Note,
) -> Result<InsertOneResult, mongodb::error::Error> {
    let insert_note: InsertOneResult = my_coll.insert_one(inserting_object).await?;
    Ok(insert_note)
}
