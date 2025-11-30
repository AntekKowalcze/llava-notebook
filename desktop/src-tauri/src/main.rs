// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Tauri command - funkcja wywoływana z frontendu
#[tauri::command]
fn greet(name: String) -> String {
    format!("Hello, {}! Welcome to Llava 🔥", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn main() {
    tauri::Builder::default()
         .plugin(tauri_plugin_devtools::init()) 
        .invoke_handler(tauri::generate_handler![
            greet  // <-- Rejestrujemy command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}