use iroh::endpoint::ConnectionError;
use tokio::task::JoinSet;

use crate::{
    crypto::encryption_stream::EncryptionStream,
    error::Result,
    event::{Event, get_event_handler},
    fs::save::FileSaver,
    p2p::{ALPN, endpoint::P2pEndpoint},
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
                    println!("Stream open");
                    match res {
                        Ok((stream_tx, stream_rx)) => {
                            // New file
                            let mut encrypted_stream = EncryptionStream::initiate(
                                false,
                                self.p2p_endpoint.secret_key(),
                                &connection.remote_node_id().unwrap(),
                                stream_tx,
                                stream_rx,
                                &ticket,
                            )
                            .await
                            .expect("noise handshake succeeds");

                            encrypted_stream.send_ready().await.unwrap();

                            let file_metadata = encrypted_stream.get_file_metadata().await.unwrap();
                            let mut file = file_saver.prepare_file(&file_metadata).await.unwrap();

                            get_event_handler().send_event(Event::PreparingFile(
                                encrypted_stream.transfer_id(),
                                file_metadata.clone(),
                                false
                            ));

                            let file_saver_c = file_saver.clone();
                            let mut total_bytes_received = 0;
                            let fut = async move {
                                loop {
                                    match encrypted_stream.recv_next_file_block(&mut file).await {
                                        Ok(0) => {
                                            file_saver_c.finish_file(&file_metadata).await.unwrap();

                                            get_event_handler().send_event(Event::TransferComplete(
                                                encrypted_stream.transfer_id()
                                            ));

                                            break;
                                        },
                                        Ok(bytes_received) => {
                                            // TODO: Ability to pause transfer
                                            total_bytes_received += bytes_received;
                                            get_event_handler().send_event(Event::TransferUpdate(
                                                encrypted_stream.transfer_id(),
                                                total_bytes_received as u64
                                            ));
                                        },
                                        Err(e) => panic!("{e}")
                                    }
                                }

                                Ok(())
                            };

                            file_streams.spawn(fut);
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
