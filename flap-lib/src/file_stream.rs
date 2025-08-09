use aead::KeyInit;
use bytes::{BufMut, BytesMut};
use iroh::endpoint::{RecvStream, SendStream, StreamId, VarInt};
use sha2::{Digest, Sha256};
use std::ops::Deref;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

use crate::crypto::master_key::MasterKey;
use crate::error::Result;
use crate::event::{Event, Progress, get_event_handler};
use crate::ticket::Ticket;

type Aead = chacha20poly1305::XChaCha20Poly1305;
type AeadEncryptor = aead::stream::EncryptorBE32<Aead>;
type AeadDecryptor = aead::stream::DecryptorBE32<Aead>;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
/// Unique per file transfer
pub struct TransferId([u8; 32]);

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

pub struct FileEncryptionStream {
    reader: File,
    aead_stream: AeadEncryptor,
    transfer_id: TransferId,
}

const ENCRYPTION_BLOCK_LENGTH: usize = 1024 * 16; // 64kB
const DECRYPTION_BLOCK_LENGTH: usize = ENCRYPTION_BLOCK_LENGTH + 16;

pub struct FileDecryptionStream {}

impl FileEncryptionStream {
    pub fn from_file(
        file: File,
        master_key: MasterKey,
        transfer_id: TransferId,
    ) -> FileEncryptionStream {
        let file_key = master_key.file_key();
        let nonce = master_key.aead_nonce();

        let file_encryption_key = file_key.get_file_encryption_key(transfer_id);
        let aead = Aead::new(file_encryption_key.deref().into());

        let aead_stream = AeadEncryptor::from_aead(aead, nonce.as_ref().into());

        Self {
            reader: file,
            aead_stream,
            transfer_id,
        }
    }

    pub async fn encrypt(mut self, stream: &mut SendStream) -> Result<()> {
        let mut bytes_encrypted = 0;
        let mut buf = [0u8; ENCRYPTION_BLOCK_LENGTH];

        loop {
            match self.reader.read(&mut buf).await {
                Ok(ENCRYPTION_BLOCK_LENGTH) => {
                    stream
                        .write_all(self.aead_stream.encrypt_next(buf.as_slice())?.as_slice())
                        .await
                        .map_err(|_| crate::error::Error::FileReadError)?;
                    bytes_encrypted += ENCRYPTION_BLOCK_LENGTH;
                }
                Ok(remaining) => {
                    stream
                        .write_all(self.aead_stream.encrypt_last(&buf[..remaining])?.as_slice())
                        .await
                        .map_err(|_| crate::error::Error::FileReadError)?;
                    bytes_encrypted += remaining;
                    break;
                }
                Err(_) => panic!("encryption failed"),
            }
        }

        get_event_handler().send_event(Event::TransferComplete(self.transfer_id));

        println!("Encrypted {bytes_encrypted} bytes");

        Ok(())
    }
}

impl FileDecryptionStream {
    // TODO: Take in a `File` and slowly write to it instead of returning bytes
    pub async fn decrypt(
        ticket: Ticket,
        file_transfer_id: TransferId,
        mut stream: RecvStream,
        file_size: u64,
    ) -> Result<()> {
        let event_handler = get_event_handler();
        let master_key = ticket.master_key();
        let file_key = master_key.file_key();

        let file_encryption_key = file_key.get_file_encryption_key(file_transfer_id);

        let nonce = master_key.aead_nonce();
        let aead = Aead::new(file_encryption_key.deref().into());

        let mut aead_stream = AeadDecryptor::from_aead(aead, nonce.as_ref().into());

        let mut file_plaintext_bytes = BytesMut::new();
        let mut buffer = BytesMut::new();
        let mut decrypted_bytes = 0;
        loop {
            // TODO: Support for unordered/parallel AEAD would allow for faster file transfer
            match stream.read_chunk(DECRYPTION_BLOCK_LENGTH, true).await {
                Ok(Some(chunk)) => {
                    buffer.put(chunk.bytes);
                    if buffer.len() >= DECRYPTION_BLOCK_LENGTH {
                        let block = buffer.split_to(DECRYPTION_BLOCK_LENGTH);

                        file_plaintext_bytes
                            .put(aead_stream.decrypt_next(block.as_ref())?.as_slice());
                        let progress = Progress::get_progress(file_size, decrypted_bytes as u64);

                        event_handler.send_event(Event::TransferUpdate(file_transfer_id, progress));

                        decrypted_bytes += DECRYPTION_BLOCK_LENGTH;
                    }
                }
                Ok(None) => {
                    // Stream is complete and we don't have a full block's amount of bytes
                    // We can therefore finish decryption
                    file_plaintext_bytes.put(aead_stream.decrypt_last(buffer.as_ref())?.as_slice());
                    decrypted_bytes += buffer.len();
                    dbg!(file_size, decrypted_bytes);
                    break;
                }
                Err(_) => panic!("decryption stream failed"),
            }
        }

        event_handler.send_event(Event::TransferComplete(file_transfer_id));

        Ok(())
    }
}
