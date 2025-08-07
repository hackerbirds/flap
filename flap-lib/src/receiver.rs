use std::env::home_dir;

use bytes::Bytes;
use iroh::endpoint::ConnectionError;
use tokio::{
    fs::{DirBuilder, File},
    io::AsyncWriteExt,
    task::JoinSet,
};

use crate::{
    error::Result,
    file_metadata::FlapFileMetadata,
    file_stream::FileDecryptionStream,
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
        let connection = self
            .p2p_endpoint
            .connect(ticket.node_id.clone(), ALPN)
            .await?;

        println!("Connection established");

        // The set of all open QUIC streams, one per file.
        let mut file_streams: JoinSet<Result<(FlapFileMetadata, Bytes)>> = JoinSet::new();

        let save_file = |file_name: String, mut file_bytes: Bytes| async move {
            let flap_dir = home_dir().expect("supported OS").join("flap-downloaded");

            // Create dir if not exists
            let _ = DirBuilder::new().create(flap_dir.clone()).await;

            let file_path = flap_dir.join(file_name);
            let mut file = File::create_new(file_path).await.unwrap();
            file.write_all_buf(&mut file_bytes).await.unwrap();
            file.flush().await.unwrap();
        };

        loop {
            tokio::select! {
                Some(Ok(res)) = file_streams.join_next() => {
                    match res {
                        Ok((file_metadata, file_bytes)) => {
                            println!("Received file {}!", file_metadata.file_name);
                            save_file(file_metadata.file_name, file_bytes).await;
                        },
                        Err(err) => panic!("err: {err:?}")
                    }
                },
                res = connection.accept_bi() => {
                    match res {
                        Ok((_stream_tx, stream_rx)) => {
                            println!("receiving new file");
                            // New file
                            let file_decr_fut = FileDecryptionStream::decrypt(ticket.clone(), stream_rx.id(), stream_rx);
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
