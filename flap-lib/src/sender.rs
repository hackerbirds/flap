use std::{
    collections::HashSet,
    io::SeekFrom,
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
};

use bytes::BytesMut;
use iroh::{
    Watcher,
    endpoint::Connection,
    protocol::{AcceptError, Router},
};
use tokio::{
    fs::File,
    io::AsyncSeekExt,
    sync::{Mutex, mpsc},
};

use crate::{
    crypto::{blake3::Blake3, encryption_stream::EncryptionStream, master_key::MasterKey},
    error::{Error, Result},
    event::{Event, get_event_handler},
    fs::metadata::FlapFileMetadata,
    p2p::{ALPN, endpoint::P2pEndpoint},
    ticket::Ticket,
};

#[derive(Debug, Clone)]
pub struct P2pSender {
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
                let (file_stream_tx, file_stream_rx) = connection.open_bi().await.unwrap();

                println!("Opened stream");

                let mut encrypted_stream = EncryptionStream::initiate(
                    true,
                    self.p2p_endpoint.secret_key(),
                    &connection.remote_node_id().unwrap(),
                    file_stream_tx,
                    file_stream_rx,
                    &self.ticket,
                )
                .await
                .expect("noise handshake succeeds");

                let file_metadata = FlapFileMetadata::from_path(&file_path).await;

                get_event_handler().send_event(Event::PreparingFile(
                    encrypted_stream.transfer_id(),
                    file_metadata.clone(),
                    true,
                ));

                encrypted_stream
                    .send_file_metadata(file_metadata)
                    .await
                    .expect("file metadata sends");

                let seek = encrypted_stream.wait_for_ready().await.expect("stream ok");

                let mut file = File::open(file_path).await.unwrap();

                if seek != 0 {
                    encrypted_stream.set_file_hasher(
                        Blake3::partial_hash(&mut file, Some(seek))
                            .await
                            .expect("File can be read and hashed"),
                    );
                    file.seek(SeekFrom::Start(seek)).await?;
                }

                let mut count = 0;
                let mut file_buf = BytesMut::zeroed(1 << 15);

                loop {
                    match encrypted_stream
                        .send_next_file_block(&mut file, &mut file_buf)
                        .await
                    {
                        Ok(0) => {
                            get_event_handler().send_event(Event::TransferComplete(
                                encrypted_stream.transfer_id(),
                            ));

                            break;
                        }
                        Ok(bytes_read) => {
                            count += bytes_read;

                            get_event_handler().send_event(Event::TransferUpdate(
                                encrypted_stream.transfer_id(),
                                count as u64,
                            ));
                        }
                        Err(e) => {
                            panic!("{e}");
                        }
                    }
                }
            }

            Ok(())
        })
    }
}
