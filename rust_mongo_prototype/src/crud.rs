// TODO write documentation for this function
pub async fn delete(
    coll: std::sync::Arc<tokio::sync::RwLock<Option<mongodb::Collection<crate::models::Note>>>>,
    note_vec: std::sync::Arc<tokio::sync::RwLock<Vec<crate::models::Note>>>,
) {
    crate::helpers::read_delete_update(coll, note_vec.clone(), |note, colls| async move {
        if let Some(a_coll) = colls.clone() {
            if let Some(note_id) = note.note_id {
                let result = crate::helpers::check_connection_and_execute_action(
                    a_coll.clone(),
                    |coll| async move {
                        println!("in deleting branch!!");
                        let filter = mongodb::bson::doc! {"_id": mongodb::bson::Bson::ObjectId(note_id)};
                        let delete = coll.delete_one(filter).await?;
                        Ok(delete)
                    },
                )
                .await;
                match result {
                    Ok(del) => println!("{:?}", del.deleted_count),
                    Err(err) => {
                        eprintln!("couldnt delete a note {err}, lost connection, try again when connection comes back");
                        let mut write = a_coll.write().await;
                        *write = None;
                    }
                }
                // if there is no connection and its not locally tell deleting onlly possible with connection
            }else {
                let mut write_vec = note_vec.write().await;                
               if let Some(index) = write_vec.iter().position(|n| *n == note) {
                     write_vec.remove(index);
}
            }
        }
    })
    .await
}
