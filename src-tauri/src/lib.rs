pub mod models;
pub mod commands;

use tauri::{
    Builder, Manager,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    Builder::default()
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            commands::pet::save_pet,
            commands::pet::list_pets,
            commands::pet::delete_pet,
            commands::settings::save_window_position,
            commands::settings::load_window_position,
            commands::generate::generate_and_assemble,
        ])
        .setup(|app| {
            let show_item = MenuItem::with_id(app, "show", "Show Pet", true, None::<&str>)?;
            let creator_item = MenuItem::with_id(app, "creator", "Open Creator", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Exit", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&show_item, &creator_item, &quit_item])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&tray_menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(win) = app.get_webview_window("pet") {
                            let _ = win.show();
                        }
                    }
                    "creator" => {
                        if let Some(win) = app.get_webview_window("creator") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error running tauri app");
}
