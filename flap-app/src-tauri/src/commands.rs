use crate::client::Client;

#[tauri::command]
pub async fn send_file(client: tauri::State<'_, Client>, file_path: String) -> Result<(), ()> {
    println!("Prepare file");
    client.send_file(file_path).await;

    Ok(())
}

#[tauri::command]
pub fn get_send_ticket(client: tauri::State<'_, Client>) -> String {
    client.ticket_string()
}

#[tauri::command]
pub async fn receive_file(
    client: tauri::State<'_, Client>,
    ticket_string: String,
) -> Result<(), ()> {
    println!("Begin receive");
    client.receive_file(ticket_string).await;

    Ok(())
}
