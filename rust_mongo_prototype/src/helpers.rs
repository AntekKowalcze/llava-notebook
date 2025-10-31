// //! module with helper functions
use crate::models::Note;
// use futures_util::stream::try_stream::TryStreamExt;
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
//zeby było uniwersalne w środku zrobić match na polączeniu czy jest czy nie ma na arc mutex, więc zmienić typ my_coll na właśnie arc mutex, które będzie clonowane przy wywołaniu jeśliej est to to co jest napisane jesli nie to sprawdzenie tyelko w wektorze,
//dalej dodać aby wyświetlał z tego vectora found tytuł oraz summary i dawał opcje wyboru po indexie + 1 vectora, i tą właśnie notatke przekazywa… do closure z funckją która ma być wykonywana, tyle że
//przy update i remove trzeba znaleźć identyczną notatke w db przez find i działać na niej/

//dodac że jeśli nie znalazło, vector jest pusty to daje błąd
pub async fn find_note_by_title_with_connection(
    my_coll: &Collection<crate::models::Note>,
    note_vec: Arc<RwLock<Vec<crate::models::Note>>>,
) -> Result<Vec<Note>, mongodb::error::Error> {
    let note_title = get_title_name();
    let mut found_notes: Vec<Note> = Vec::new();
    let vec_read = note_vec.read().await;
    for note in vec_read.iter() {
        if note.title == note_title {
            found_notes.push(note.clone());
        }
    }

    let filter = mongodb::bson::doc! {"title": &note_title};
    let mut coursor = my_coll.find(filter).await?;
    while let Some(note) = coursor.try_next().await? {
        println!("adding from db");
        found_notes.push(note);
    }
    println!("found notes: {found_notes :?}");
    Ok(found_notes)
}

fn get_title_name() -> String {
    println!("Type note title:");
    let mut note_title = String::new();
    std::io::stdin()
        .read_line(&mut note_title)
        .expect("input cant be red");
    note_title.trim().to_string()
}

//read prompt note title to find note check first in locally cuz its faster, then check into database by title and if the same title exist show both summaries and tell to pick, show it
//wykonywać to w helperze który sprawdza połączenie w zależności od tego, dać w result error sprawdzanie lokalne i osobną funkcje do wybierania
// helper, musi brać arc rwlock dla połaczenia, musi prac arc rwlock vector obie rzeczy zklonowane, robić read i odrazu drop, jeśli połączenie sprawdzić i lokalnie w vec i w połączeniu jeśl nie komunikat że bedzie tylko z lokalnych notatek
//jeśli identyczne tytułu pobrać dla wszystich summary i wyświetlić z numerami, przypiasć numery do summary żeby się to jakoś dało odróżnić od siebie
// musi robić deserializacje do structa lub jakoś inaczej to wyświetlać (sprawdzić)
