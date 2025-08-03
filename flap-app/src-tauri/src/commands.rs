use flap_lib::{receiver::P2pReceiver, sender::P2pSender};
use std::{fs::File, io::Write};

#[tauri::command]
pub async fn send_file(file_path: String) -> String {
    let sender = P2pSender::new().await.unwrap();

    let file = File::open(file_path).unwrap();
    let ticket = sender.send(file).await.unwrap();

    ticket.convert()
}

#[tauri::command]
pub async fn receive_file(ticket_string: String) -> Result<(), ()> {
    let receiver = P2pReceiver::new().await.unwrap();
    let ticket = ticket_string.parse().unwrap();
    let mut retrieved_bytes = receiver.retrieve(ticket).await.unwrap();
    let mut save_path = std::env::home_dir().unwrap();
    save_path.push("flapped-file.txt");
    
    let mut save_file = File::create(save_path).unwrap();
    save_file.write_all(&mut retrieved_bytes).unwrap();

    Ok(())
}
