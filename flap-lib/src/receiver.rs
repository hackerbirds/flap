use iroh_blobs::{api::downloader::Downloader, store::mem::MemStore};

use crate::{error::Result, file_stream::FileDecryptionStream, p2p::P2pEndpoint, ticket::Ticket};

#[derive(Debug)]
pub struct P2pReceiver {
    #[expect(dead_code)]
    p2p_endpoint: P2pEndpoint,
    store: MemStore,
    downloader: Downloader,
}

impl P2pReceiver {
    pub async fn new() -> Result<Self> {
        let p2p_endpoint = P2pEndpoint::start().await?;
        let store = MemStore::new();
        let downloader = store.downloader(&p2p_endpoint);

        Ok(Self {
            p2p_endpoint,
            store,
            downloader,
        })
    }

    pub async fn retrieve(&self, ticket: Ticket) -> Result<Vec<u8>> {
        let blob_hash = ticket.blob_ticket.hash();
        self.downloader
            .download(blob_hash, Some(ticket.blob_ticket.node_addr().node_id))
            .await
            .unwrap();

        let blob_reader = self.store.reader(blob_hash);
        let file_decryptor = FileDecryptionStream::new(blob_reader, ticket.master_key());
        let bytes = file_decryptor.decrypt().await?.to_vec();

        println!("Received (bytes): {:?}", &bytes);
        println!("Received: {}", String::from_utf8(bytes.clone()).unwrap());

        Ok(bytes)
    }
}
