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
    io::AsyncWriteExt,
    sync::{Mutex, mpsc},
};

use crate::{
    crypto::master_key::MasterKey,
    error::{Error, Result},
    file_stream::{FileEncryptor, TransferId},
    fs::metadata::FlapFileMetadata,
    p2p::{ALPN, P2pEndpoint},
    ticket::Ticket,
};

#[derive(Debug, Clone)]
pub struct P2pSender {
    #[expect(dead_code)]
    p2p_endpoint: P2pEndpoint,
    files_added: Arc<Mutex<HashSet<PathBuf>>>,
    files_queue_tx: mpsc::UnboundedSender<PathBuf>,
    // We don't use it directly but it stores running connections
    #[allow(dead_code)]
    router: Router,
    pub ticket: Ticket,
}

#[derive(Debug, Clone)]
pub struct P2pSenderHandler {
    pub ticket: Ticket,
    files_queue_rx: Arc<Mutex<mpsc::UnboundedReceiver<PathBuf>>>,
}

impl P2pSender {
    pub async fn new() -> Result<Self> {
        let p2p_endpoint = P2pEndpoint::start().await?;
        let node_addr = p2p_endpoint.node_addr().initialized().await;

        let ticket = Ticket::make(node_addr.node_id, MasterKey::generate());

        let (files_queue_tx, files_queue_rx) = mpsc::unbounded_channel();
        let files_queue_rx = Arc::new(Mutex::new(files_queue_rx));

        let p2p_sender_handler = P2pSenderHandler {
            ticket: ticket.clone(),
            files_queue_rx,
        };

        let router = Router::builder(p2p_endpoint.deref().clone())
            .accept(ALPN, p2p_sender_handler)
            .spawn();

        let files_added = Arc::new(Mutex::new(HashSet::new()));

        println!("endpoint set up ok");

        Ok(Self {
            p2p_endpoint,
            files_queue_tx,
            files_added,
            router,
            ticket,
        })
    }

    pub async fn send(&self, path: impl AsRef<Path>) -> Result<()> {
        let file_path = path.as_ref().to_path_buf();
        if self.files_added.lock().await.insert(file_path.clone()) {
            println!("Sending file {file_path:?}");
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

impl iroh::protocol::ProtocolHandler for P2pSenderHandler {
    fn accept(
        &self,
        connection: Connection,
    ) -> impl Future<Output = std::result::Result<(), AcceptError>> + Send {
        println!("New connection established");
        let files_queue_rx = self.files_queue_rx.clone();

        Box::pin(async move {
            // TODO: Use a JoinSet or something because this is only one file at a time
            while let Some(file_path) = files_queue_rx.lock().await.recv().await {
                let file_metadata = FlapFileMetadata::load(&file_path).await;
                let file_metadata_bytes = file_metadata.to_bytes();

                let file = File::open(file_path).await?;
                println!("New file found. Creating new QUIC stream");
                let (mut file_stream_tx, _file_stream_rx) = connection.open_bi().await.unwrap();

                let file_transfer_id = TransferId::new(&self.ticket, file_stream_tx.id());
                let file_stream =
                    FileEncryptor::from_file(file, *self.ticket.master_key(), file_transfer_id);

                println!("Sending file metadata...");
                file_stream_tx
                    .write_u64(file_metadata_bytes.len() as u64)
                    .await
                    .unwrap();
                file_stream_tx
                    .write_all(&file_metadata_bytes)
                    .await
                    .unwrap();

                println!("Begin to encrypting file...");
                file_stream.encrypt(&mut file_stream_tx).await.unwrap();
                println!("Encrypting file done! Other stream received the complete encrypted file");
                file_stream_tx.finish().unwrap();
            }

            Ok(())
        })
    }
}
