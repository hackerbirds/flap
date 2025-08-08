use std::{
    fmt::Display,
    sync::{Arc, OnceLock},
};

use tokio::sync::{Mutex, MutexGuard, mpsc};

use crate::{file_metadata::FlapFileMetadata, file_stream::TransferId};

/// A value between 0 and 100.
#[derive(Debug, Clone, Copy)]
pub struct Progress(f64);

impl Progress {
    pub const fn zero() -> Self {
        Progress(0f64)
    }

    pub fn get_progress(total_file_size: u64, decrypted_bytes: u64) -> Self {
        Progress(100f64 * (decrypted_bytes as f64) / (total_file_size as f64))
    }
}

impl Display for Progress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}%", self.0)
    }
}

impl AsRef<f64> for Progress {
    fn as_ref(&self) -> &f64 {
        &self.0
    }
}

#[derive(Debug)]
pub enum Event {
    TransferUpdate(TransferId, Progress),
    ReceivingFile(TransferId, FlapFileMetadata),
    TransferComplete(TransferId),
}

static EVENT_HANDLER: OnceLock<EventHandler> = OnceLock::new();

pub fn get_event_handler() -> &'static EventHandler {
    EVENT_HANDLER.get_or_init(EventHandler::new)
}

/// Allows for asynchronous updates to the front-end.
/// Can be cloned cheaply.
#[derive(Clone)]
pub struct EventHandler {
    sender: mpsc::UnboundedSender<Event>,
    receiver: Arc<Mutex<mpsc::UnboundedReceiver<Event>>>,
}

impl EventHandler {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let receiver = Arc::new(Mutex::new(receiver));

        Self { sender, receiver }
    }

    pub fn send_event(&self, event: Event) {
        self.sender
            .send(event)
            .expect("Receiver is open while EventHandler exists")
    }

    pub async fn get_receiver(&self) -> MutexGuard<'_, mpsc::UnboundedReceiver<Event>> {
        self.receiver.lock().await
    }
}
