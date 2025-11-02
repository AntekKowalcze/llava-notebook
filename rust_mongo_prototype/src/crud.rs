//! module responsible for crud operations, here its mainly delete and update



/// function responsible for deleteing a note
/// # Arguments
///  *coll - arc rwlock to option to collection of notes
/// 
///  *note_vec arc rwlock to local note vec

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

pub async fn update(
 coll: std::sync::Arc<tokio::sync::RwLock<Option<mongodb::Collection<crate::models::Note>>>>,
 note_vec: std::sync::Arc<tokio::sync::RwLock<Vec<crate::models::Note>>>,
){
crate::helpers::read_delete_update(coll, note_vec.clone(), |note, colls| async move {   
        if let Some(a_coll) = colls.clone() {
            let b_coll = a_coll.clone();
            if let Some(note_id) = note.note_id {
                let result = crate::helpers::check_connection_and_execute_action(
                    a_coll.clone(),
                    |coll| async move {
                         let field = crate::helpers::get_note_field();
                         let filter = mongodb::bson::doc! {"_id": mongodb::bson::Bson::ObjectId(note_id)};
                       let number_of_modified =  if let Some(note) = coll.find_one(filter).await?{
                             let update =  match field{
                            1 => {
                                let content = note.title;
                                let updated_content = crate::helpers::edit_note(content).expect("problem with value editor crate");

                                mongodb::bson::doc! { "$set": mongodb::bson::doc! {"title": updated_content} }
                            },
                            2 =>  {
                                let content = note.summary;
                                let updated_content = crate::helpers::edit_note(content).expect("problem with value editor crate");

                                mongodb::bson::doc! { "$set": mongodb::bson::doc! {"summary": updated_content} }},
                            3 =>  {let content = note.content;
                                let updated_content = crate::helpers::edit_note(content).expect("problem with value editor crate");
                                mongodb::bson::doc! { "$set": mongodb::bson::doc! {"content": updated_content} }
                                

                            },
                            _ =>  {
                                println!("option to high, updating content");
                                let content = note.content;
                                let updated_content = crate::helpers::edit_note(content).expect("problem with value editor crate");
                                mongodb::bson::doc! { "$set": mongodb::bson::doc! {"content": updated_content} }},
                                
                         };
                           let filter = mongodb::bson::doc! {"_id": mongodb::bson::Bson::ObjectId(note_id)};
                           let res = coll.update_one(filter, update).await?;
                           println!("updated count {:?}", res.modified_count);
                           res.modified_count
                         }else{
                            eprintln!("note not found, propably lost connection");
                             let mut write = a_coll.write().await;
                            *write = None;
                           let zero: u64 = 0;
                           zero

                         };
                         
                        Ok(number_of_modified)
                           
                        
                            
                       
                    },
                    
                )
                .await;
            
                match result {
                    Ok(number_of_modified) => println!("number of modified: {}", number_of_modified),
                    Err(err) => {
                        eprintln!("couldnt update a note {err}, lost connection, try again when connection comes back");
                        let mut write = b_coll.write().await;
                        *write = None;
                    }
                }
                // if there is no connection and its not locally tell deleting onlly possible with connection
            }else {
                let wv = note_vec.clone();
                let mut write_vec: tokio::sync::RwLockWriteGuard<'_, Vec<crate::models::Note>> = note_vec.write().await;                
               if let Some(index) = write_vec.iter().position(|n| *n == note) {
                  let mut note = write_vec.remove(index);
                  drop(write_vec);
                  let field = crate::helpers::get_note_field();
                  match field{
                    1 => {
                        let content = note.title;
                        let updated_content = crate::helpers::edit_note(content).expect("problem with value editor crate");
                         note.title = updated_content;

                    },
                    2 => {
                        let content = note.summary;
                        let updated_content = crate::helpers::edit_note(content).expect("problem with value editor crate");
                         note.summary= updated_content;
                    },
                    _ => {
                        println!("choosen 3 or bigger number, updating content");
                        let content = note.content;
                        let updated_content = crate::helpers::edit_note(content).expect("problem with value editor crate");
                        note.content = updated_content;
                    },}
                    crate::save_locally::save_locally(note, wv).await; 
            }
        }
}}).await
}
//to update we have to choose note, choose field, 