use std::fs::File;

use iroh::protocol::Router;
use iroh_blobs::{BlobsProtocol, store::mem::MemStore};

use crate::{
    crypto::master_key::MasterKey, error::Result, file_stream::FileEncryptionStream,
    p2p::P2pEndpoint, ticket::Ticket,
};

#[derive(Debug)]
pub struct P2pSender {
    #[expect(dead_code)]
    p2p_endpoint: P2pEndpoint,
    store: MemStore,
    blobs: BlobsProtocol,
    master_key: MasterKey,
}

impl P2pSender {
    pub async fn new() -> Result<Self> {
        let p2p_endpoint = P2pEndpoint::start().await?;
        let store = MemStore::new();

        let blobs = BlobsProtocol::new(&store, p2p_endpoint.clone(), None);

        let master_key = MasterKey::generate();

        let router = Router::builder(p2p_endpoint.clone())
            .accept(iroh_blobs::ALPN, blobs.clone())
            .spawn();

        // Leave router on "forever"
        Box::leak(Box::new(router));

        Ok(Self {
            p2p_endpoint,
            store,
            blobs,
            master_key,
        })
    }

    // for now file is just a string
    pub async fn send(&self, file: File) -> Result<Ticket> {
        let file_stream = FileEncryptionStream::from_file(file, self.master_key);
        // turn the first await into a poll loop to have upload bar
        let file_tag = self.store.blobs().add_stream(file_stream).await.await?;

        let blob_ticket = self.blobs.ticket(file_tag).await.unwrap();

        let ticket = Ticket::make(blob_ticket, self.master_key);

        Ok(ticket)
    }
}
