use std::{
    collections::HashSet,
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
};

use iroh::{
    Watcher,
    endpoint::Connection,
    protocol::{AcceptError, Router},
};
use tokio::{
    fs::File,
    sync::{Mutex, mpsc},
};

use crate::{
    crypto::{master_key::MasterKey, transfer_id::TransferId},
    error::{Error, Result},
    event::{Event, get_event_handler},
    file_stream::FileEncryptor,
    fs::metadata::FlapFileMetadata,
    p2p::{ALPN, P2pEndpoint},
    ticket::Ticket,
};

#[derive(Debug, Clone)]
pub struct P2pSender {
    #[expect(dead_code)]
    p2p_endpoint: P2pEndpoint,
    files_queue_tx: mpsc::UnboundedSender<PathBuf>,
    files_queue_rx: Arc<Mutex<mpsc::UnboundedReceiver<PathBuf>>>,
    files_added: Arc<Mutex<HashSet<PathBuf>>>,
    pub ticket: Ticket,
}

impl P2pSender {
    pub async fn new() -> Result<Self> {
        let p2p_endpoint = P2pEndpoint::start().await?;
        let node_addr = p2p_endpoint.node_addr().initialized().await;

        let ticket = Ticket::make(node_addr.node_id, MasterKey::generate());

        let (files_queue_tx, files_queue_rx) = mpsc::unbounded_channel();
        let files_queue_rx = Arc::new(Mutex::new(files_queue_rx));
        let files_added = Arc::new(Mutex::new(HashSet::new()));
        let p2p_sender = Self {
            p2p_endpoint: p2p_endpoint.clone(),
            files_queue_tx,
            files_queue_rx,
            files_added,
            ticket,
        };

        let router = Router::builder(p2p_endpoint.deref().clone())
            .accept(ALPN, p2p_sender.clone())
            .spawn();

        // Hack to leave router running
        Box::leak(Box::new(router));

        Ok(p2p_sender)
    }

    pub async fn send(&self, path: impl AsRef<Path>) -> Result<()> {
        let file_path = path.as_ref().to_path_buf();

        if self.files_added.lock().await.insert(file_path.clone()) {
            self.files_queue_tx.send(file_path).map_err(|_| {
                println!("error happened while preparing to send file");
                Error::MpscSendError
            })?;

            Ok(())
        } else {
            Err(Error::FileAlreadyAdded)
        }
    }
}

impl iroh::protocol::ProtocolHandler for P2pSender {
    fn accept(
        &self,
        connection: Connection,
    ) -> impl Future<Output = std::result::Result<(), AcceptError>> + Send {
        let files_queue_rx = self.files_queue_rx.clone();
        Box::pin(async move {
            while let Some(file_path) = files_queue_rx.lock().await.recv().await {
                let (mut file_stream_tx, _file_stream_rx) = connection.open_bi().await.unwrap();

                let file_metadata = FlapFileMetadata::from_path(&file_path).await;
                let file = File::open(file_path).await.unwrap();

                let file_transfer_id = TransferId::new(&self.ticket, file_stream_tx.id());
                let file_stream =
                    FileEncryptor::from_file(file, *self.ticket.master_key(), file_transfer_id);

                get_event_handler().send_event(Event::PreparingFile(
                    file_transfer_id,
                    file_metadata.clone(),
                    true,
                ));

                file_stream
                    .encrypt(file_metadata, &mut file_stream_tx)
                    .await
                    .unwrap();

                file_stream_tx.finish().unwrap();
            }

            Ok(())
        })
    }
}
