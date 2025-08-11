use iroh::endpoint::ConnectionError;
use tokio::task::JoinSet;

use crate::{
    crypto::transfer_id::TransferId,
    error::Result,
    file_stream::FileDecryptor,
    fs::save::FileSaver,
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

        println!("Connection established");

        let file_saver = FileSaver::new().await;

        // The set of all file decryptor streams.
        let mut file_streams: JoinSet<Result<()>> = JoinSet::new();

        loop {
            tokio::select! {
                Some(Ok(res)) = file_streams.join_next() => {
                    match res {
                        Ok(()) => {
                            println!("File downloaded and saved successfully.");
                        },
                        Err(err) => panic!("err: {err:?}")
                    }
                },
                res = connection.accept_bi() => {
                    match res {
                        Ok((_stream_tx, stream_rx)) => {
                            // New file
                            let file_transfer_id = TransferId::new(&ticket, stream_rx.id());

                            file_streams.spawn(FileDecryptor::launch(ticket.clone(), file_transfer_id, stream_rx, file_saver.clone()));
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
