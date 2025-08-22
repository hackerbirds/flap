use tauri::{async_runtime, Manager};

use crate::client::Client;

pub mod client;
pub mod commands;
pub mod frontend_events;

pub struct AppState {
    pub client: Client,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        //.plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handler = app.handle().clone();
            async_runtime::spawn(async move {
                println!("Setting up client");

                let client = Client::start(handler.clone()).await;
                handler.manage(client);
                println!("Client has been set up");
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::send_file,
            commands::receive_file,
            commands::get_send_ticket,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
