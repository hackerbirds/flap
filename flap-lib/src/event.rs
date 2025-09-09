use std::sync::{Arc, OnceLock};

use tokio::sync::{Mutex, MutexGuard, mpsc};

#[cfg(feature = "tracing")]
use tracing::info;

use crate::{crypto::transfer_id::TransferId, fs::metadata::FlapFileMetadata};

#[derive(Debug)]
pub enum Event {
    TransferUpdate(TransferId, u64),
    PreparingFile(TransferId, FlapFileMetadata, bool /* sending? */),
    TransferComplete(TransferId),
}

static EVENT_HANDLER: OnceLock<EventHandler> = OnceLock::new();

pub fn get_event_handler() -> &'static EventHandler {
    EVENT_HANDLER.get_or_init(EventHandler::new)
}

/// Allows for asynchronous updates to the front-end.
/// Can be cloned cheaply.
#[derive(Debug, Clone)]
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

    // #[cfg_attr(feature = "tracing", instrument)]
    pub fn send_event(&self, event: Event) {
        #[cfg(feature = "tracing")]
        info!("Sending event {:?}", &event);

        self.sender
            .send(event)
            .expect("Receiver is open while EventHandler exists")
    }

    pub async fn get_receiver(&self) -> MutexGuard<'_, mpsc::UnboundedReceiver<Event>> {
        self.receiver.lock().await
    }
}
