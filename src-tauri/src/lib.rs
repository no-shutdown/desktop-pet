pub mod models;
pub mod commands;

use tauri::{
    AppHandle, Builder, Manager,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};

fn open_or_create_creator(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("creator") {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
    } else {
        use tauri::WebviewWindowBuilder;
        if let Ok(win) = WebviewWindowBuilder::new(
            app,
            "creator",
            tauri::WebviewUrl::App("index.html".into()),
        )
        .title("Desktop Pet — Create")
        .inner_size(860.0, 640.0)
        .resizable(true)
        .build()
        {
            let _ = win.set_focus();
        }
    }
}

#[tauri::command]
fn open_creator(app: AppHandle) {
    open_or_create_creator(&app);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    Builder::default()
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            open_creator,
            commands::pet::save_pet,
            commands::pet::list_pets,
            commands::pet::delete_pet,
            commands::settings::save_window_position,
            commands::settings::load_window_position,
            commands::generate::generate_and_assemble,
            commands::generate::save_custom_frames,
            commands::plugin::scan_plugins,
            commands::plugin::read_plugin_file,
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
                            let _ = win.unminimize();
                            let _ = win.show();
                            let _ = win.set_focus();
                            // Tell the pet window to refresh its pet list
                            use tauri::Emitter;
                            let _ = win.emit("pet-window-show", ());
                        }
                    }
                    "creator" => open_or_create_creator(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            // Start local HTTP event receiver for Claude Code hooks
            let event_app = app.handle().clone();
            std::thread::spawn(move || {
                use tauri::Emitter;

                let server = match tiny_http::Server::http("127.0.0.1:29513") {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("[event-server] Failed to bind port 29513: {e}");
                        return;
                    }
                };
                for mut request in server.incoming_requests() {
                    let mut body = String::new();
                    request.as_reader().read_to_string(&mut body).ok();
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                        if let Some(event_type) = json.get("type").and_then(|v| v.as_str()) {
                            let _ = event_app.emit("plugin-event", event_type.to_string());
                        }
                    }
                    let _ = request.respond(tiny_http::Response::from_string("ok"));
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error running tauri app");
}
