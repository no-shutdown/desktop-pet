pub mod models;
pub mod commands;

use tauri::Builder;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    Builder::default()
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            commands::pet::save_pet,
            commands::pet::list_pets,
            commands::pet::delete_pet,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
