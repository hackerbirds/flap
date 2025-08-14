use iroh::endpoint::{StreamId, VarInt};
use sha2::{Digest, Sha256};

use crate::ticket::Ticket;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
/// Unique per file transfer
pub struct TransferId(pub [u8; 32]);

impl TransferId {
    pub fn new(ticket: &Ticket, stream_id: StreamId) -> TransferId {
        let mut hasher = Sha256::new();
        hasher.update(&ticket.master_key().0);
        hasher.update(ticket.node_id.as_bytes());
        hasher.update(&VarInt::from(stream_id).into_inner().to_le_bytes());

        let hash = hasher.finalize();
        TransferId(hash.into())
    }
}

impl AsRef<[u8]> for TransferId {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}
