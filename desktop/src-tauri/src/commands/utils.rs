use std::time::Duration;

use anyhow::anyhow;
use llava_core::AppState;
use tauri::AppHandle;
use tauri::Emitter;
use tauri::Manager;
use tauri::async_runtime;
#[tauri::command]
pub async fn get_username_from_uuid(
    user_uuid: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, llava_core::Error> {
    let users_db_guard = state
        .users_db
        .lock()
        .map_err(|_| anyhow!("error while gettnig users_db from state"))?;
    let users_db = users_db_guard
        .as_ref()
        .ok_or(llava_core::Error::LockError)?;
    let username = llava_core::get_username_from_uuid(users_db, user_uuid)?;
    Ok(username)
}

pub fn start_connection_monitor(app_handle: AppHandle) {
    let mut fail_count = 0u32;

    async_runtime::spawn(async move {
        
        let state = app_handle.state::<AppState>();
        
        let mut ticker = tokio::time::interval(Duration::from_secs(5));
        ticker.tick().await;

        loop {
          
            let is_local_only: bool = match state.user_config.lock() {
            Ok(config) => config
                .as_ref()
                .and_then(|map| map.get("local.mode")) 
                .map(|value| value == "on")
                .unwrap_or(false),
            Err(_) => true,
        };  
        if !is_local_only{
            match llava_core::check_connection().await {
                Ok((server, internet)) => {
                    let _ = app_handle.emit("server_connection_status", server);
                    let _ = app_handle.emit("internet_connection_status", internet);
                    if let Ok(mut lock) = state.server_connection.lock() {
                        *lock = server;
                    }
                    if let Ok(mut lock) = state.internet_connection.lock() {
                        *lock = internet;   
                    }
                    fail_count = 0;
                }
                Err(_) => {
                    fail_count += 1;
                    if fail_count >= 3 {
                        tokio::time::sleep(Duration::from_secs(45)).await;
                        fail_count = 0;
                    }
                    if let Ok(mut lock) = state.server_connection.lock() {
                        *lock = false;
                    }
                    if let Ok(mut lock) = state.internet_connection.lock() {
                        *lock = false;
                    }
                }
            }
        }
          ticker.tick().await;
        }
    });
}

pub fn check_connection_before_request(state: tauri::State<'_, AppState>) -> Result<(), llava_core::Error> {
   let is_online = {
       let guard =  state.internet_connection.lock().map_err(|_| llava_core::Error::LockError)?;
        guard.clone()
    };
    let is_connected_to_server = {
let guard =  state.server_connection.lock().map_err(|_| llava_core::Error::LockError)?;
        guard.clone()
    };
    if !is_online {
        return Err(llava_core::Error::NoInternetConnection)
    }
    if !is_connected_to_server {
    return Err(llava_core::Error::ServerNotAvailable)

    }
    Ok(())
}