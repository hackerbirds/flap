use flap_lib::{receiver::P2pReceiver, sender::P2pSender};
use tauri::Manager;

pub mod commands;

pub struct AppState {
    pub p2p_sender: P2pSender,
    pub p2p_receiver: P2pReceiver,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        //.plugin(tauri_plugin_dialog::init())
        //.plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handler = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                println!("Setting up P2P");

                let p2p_sender = P2pSender::new().await.unwrap();
                let p2p_receiver = P2pReceiver::new().await.unwrap();

                let app_state = AppState {
                    p2p_sender,
                    p2p_receiver,
                };

                assert!(handler.manage(app_state));

                println!("P2P Has been set up");
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::send_file,
            commands::receive_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
