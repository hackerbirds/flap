use std::env::home_dir;

use bytes::BytesMut;
use iroh::endpoint::ConnectionError;
use tokio::{io::AsyncReadExt, task::JoinSet};

use crate::{
    error::Result,
    event::{Event, get_event_handler},
    file_metadata::FlapFileMetadata,
    file_stream::{FileDecryptionStream, TransferId},
    p2p::{ALPN, P2pEndpoint},
    ticket::Ticket,
};

#[derive(Debug)]
pub struct P2pReceiver {
    p2p_endpoint: P2pEndpoint,
}

impl P2pReceiver {
    pub async fn new() -> Result<Self> {
        let p2p_endpoint = P2pEndpoint::start().await?;

        Ok(Self { p2p_endpoint })
    }

    pub async fn retrieve(&self, ticket: Ticket) -> Result<()> {
        println!("Establishing connection");

        let connection = self
            .p2p_endpoint
            .connect(ticket.node_id.clone(), ALPN)
            .await?;

        let event_handler = get_event_handler();

        println!("Connection established");

        // The set of all open QUIC streams, one per file.
        let mut file_streams: JoinSet<Result<()>> = JoinSet::new();

        loop {
            tokio::select! {
                Some(Ok(res)) = file_streams.join_next() => {
                    match res {
                        Ok(()) => {
                            println!("File downloaded successfully");
                        },
                        Err(err) => panic!("err: {err:?}")
                    }
                },
                res = connection.accept_bi() => {
                    match res {
                        Ok((_stream_tx, mut stream_rx)) => {
                            // New file
                            let file_transfer_id = TransferId::new(&ticket, stream_rx.id());

                            let file_metadata_info_length = stream_rx
                                .read_u64()
                                .await
                                .map_err(|_| crate::error::Error::FileReadError)?;

                            let mut file_metadata_bytes = BytesMut::zeroed(file_metadata_info_length as usize);
                            stream_rx.read_exact(&mut file_metadata_bytes).await.unwrap();
                            let file_metadata = FlapFileMetadata::from_bytes(file_metadata_bytes.into()).await;
                            let file_size = file_metadata.file_size;
                            event_handler.send_event(Event::ReceivingFile(file_transfer_id, file_metadata));

                            let file_decr_fut = FileDecryptionStream::decrypt(ticket.clone(), file_transfer_id, stream_rx, file_size);
                            file_streams.spawn(file_decr_fut);
                        },
                        Err(ConnectionError::LocallyClosed) => { println!("Stream closed") }
                        Err(err) => {
                            println!("Something strange happeend while accepting stream: {err:?}");
                            break;
                        }
                    }
                },
                else => {
                    println!("Nothing happening");
                }
            }
        }

        Ok(())
    }
}
