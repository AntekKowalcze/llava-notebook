// //! module with helper functions
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
