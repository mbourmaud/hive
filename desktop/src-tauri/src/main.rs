#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use std::net::TcpListener;

fn get_available_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to random port");
    listener.local_addr().unwrap().port()
}

fn main() {
    let port = get_available_port();

    tauri::Builder::default()
        .setup(move |app| {
            // Spawn the axum server in background
            let _handle = tauri::async_runtime::spawn(async move {
                if let Err(e) = hive_lib::webui::start_server_async(port).await {
                    eprintln!("Server error: {}", e);
                }
            });

            // Navigate the main window to the server
            if let Some(window) = app.get_webview_window("main") {
                let url = format!("http://localhost:{}", port);
                let _ = window.navigate(url.parse().unwrap());
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::stop_drone,
            commands::clean_drone,
            commands::list_plans,
            commands::start_drone,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
