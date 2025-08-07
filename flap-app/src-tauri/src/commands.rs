use crate::AppState;

#[tauri::command]
pub async fn send_file(state: tauri::State<'_, AppState>, file_path: String) -> Result<String, ()> {
    println!("Prepare file");
    let sender = &state.p2p_sender;
    sender.send(file_path).await.unwrap();
    let ticket = sender.ticket.convert();

    Ok(ticket)
}

#[tauri::command]
pub async fn receive_file(
    state: tauri::State<'_, AppState>,
    ticket_string: String,
) -> Result<(), ()> {
    let receiver = &state.p2p_receiver;

    let ticket = ticket_string.parse().unwrap();
    receiver.retrieve(ticket).await.unwrap();

    Ok(())
}
