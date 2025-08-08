use flap_lib::{
    event::{get_event_handler, Event},
    receiver::P2pReceiver,
    sender::P2pSender,
};
use tauri::{async_runtime, AppHandle, Emitter};

use crate::frontend_events;

pub struct Client {
    p2p_sender: P2pSender,
    p2p_receiver: P2pReceiver,
    #[expect(dead_code)]
    tauri_app_handle: AppHandle,
}

impl Client {
    pub async fn start(tauri_app_handle: AppHandle) -> Self {
        let p2p_sender = P2pSender::new().await.unwrap();
        let p2p_receiver = P2pReceiver::new().await.unwrap();

        let tauri_app_handle_c = tauri_app_handle.clone();

        let client = Self {
            p2p_sender,
            p2p_receiver,
            tauri_app_handle,
        };

        // The running background task which responds to events and updates UI/etc. accordingly.
        let _event_task = async_runtime::spawn(async move {
            let event_handler = get_event_handler();
            while let Some(event) = event_handler.get_receiver().await.recv().await {
                match event {
                    Event::TransferUpdate(file_transfer_id, progress) => {
                        tauri_app_handle_c
                            .emit(
                                "transfer-update",
                                frontend_events::TransferUpdateEvent {
                                    file_transfer_id: file_transfer_id.as_ref().to_vec(),
                                    progress: (*progress.as_ref()) as u64,
                                },
                            )
                            .unwrap();
                    }
                    Event::ReceivingFile(file_transfer_id, flap_file_metadata) => {
                        let file_size = flap_file_metadata.file_size;
                        let file_name = flap_file_metadata.file_name;
                        println!("{file_transfer_id:?}");
                        tauri_app_handle_c
                            .emit(
                                "receiving-file",
                                frontend_events::ReceivingFileEvent {
                                    file_transfer_id: file_transfer_id.as_ref().to_vec(),
                                    metadata: frontend_events::FileMetadata {
                                        file_name,
                                        file_size,
                                    },
                                },
                            )
                            .unwrap();
                    }
                    Event::TransferComplete(file_transfer_id) => {
                        tauri_app_handle_c
                            .emit(
                                "transfer-complete",
                                frontend_events::TransferCompleteEvent {
                                    file_transfer_id: file_transfer_id.as_ref().to_vec(),
                                },
                            )
                            .unwrap();
                    }
                }
            }
        });

        client
    }

    pub fn ticket_string(&self) -> String {
        self.p2p_sender.ticket.convert()
    }

    pub async fn send_file(&self, file_path: String) {
        self.p2p_sender.send(file_path).await.unwrap();
    }

    pub async fn receive_file(&self, ticket_string: String) {
        let ticket = ticket_string.parse().unwrap();

        self.p2p_receiver.retrieve(ticket).await.unwrap();
    }
}
