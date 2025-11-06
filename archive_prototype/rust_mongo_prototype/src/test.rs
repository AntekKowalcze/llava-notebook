// #[cfg(test)]
// #[tokio::test]
// #[ignore]async fn test() {
//     use tokio::sync::RwLock;

//     use crate::models::Note;
//     use std::sync::Arc;
//     let note1 = Note {
//         created_at: "2025-10-31T22:00:00Z".to_string(),
//         title: "Rust Tips".to_string(),
//         summary: "Learning trait bounds".to_string(),
//         content: "Remember to import TryStreamExt for MongoDB cursor".to_string(),
//     };

//     let note2 = Note {
//         created_at: "2025-10-31T21:45:00Z".to_string(),
//         title: "Study".to_string(),
//         summary: "Fix MongoDB connection".to_string(),
//         content: "Add trait bounds to generic type parameters".to_string(),
//     };

//     let note3 = Note {
//         created_at: "2025-10-31T20:30:00Z".to_string(),
//         title: "Study".to_string(),
//         summary: "Cursor error fixed".to_string(),
//         content: "try_next() now works after import fix".to_string(),
//     };

//     let note4 = Note {
//         created_at: "2025-10-31T19:15:00Z".to_string(),
//         title: "test".to_string(),
//         summary: "Generic CRUD functions".to_string(),
//         content: "Always add Send + Sync bounds for async code".to_string(),
//     };

//     let note5 = Note {
//         created_at: "2025-10-31T18:00:00Z".to_string(),
//         title: "test".to_string(),
//         summary: "Auto-traits in Rust".to_string(),
//         content: "Send, Sync, Unpin auto-derive when fields match".to_string(),
//     };
//     let v1 = crate::db::connecting_to_db().await.unwrap();
//     let v2 = v1.clone();
//     let v: Arc<RwLock<Option<mongodb::Collection<Note>>>> = Arc::new(RwLock::new(Some(v1)));
//     let vec: Vec<Note> = vec![note1, note2.clone(), note3.clone(), note4, note5];
//     for note in &vec {
//         crate::db::inserting_note(&v2, note).await.unwrap();
//     }
//     let vec: Arc<RwLock<Vec<crate::models::Note>>> = Arc::new(RwLock::new(vec));
//     let test_vec = crate::helpers::find_note_by_title(v.clone(), vec)
//         .await
//         .unwrap();
//     assert_eq!(test_vec.unwrap(), note2);
// }

//
