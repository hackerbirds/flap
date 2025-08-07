use aead::KeyInit;
use bytes::{BufMut, Bytes, BytesMut};
use iroh::endpoint::{RecvStream, SendStream, StreamId};
use std::ops::Deref;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

use crate::crypto::master_key::MasterKey;
use crate::error::Result;
use crate::file_metadata::FlapFileMetadata;
use crate::ticket::Ticket;

type Aead = chacha20poly1305::XChaCha20Poly1305;
type AeadEncryptor = aead::stream::EncryptorBE32<Aead>;
type AeadDecryptor = aead::stream::DecryptorBE32<Aead>;

pub struct FileEncryptionStream {
    reader: File,
    aead_stream: AeadEncryptor,
}

const ENCRYPTION_BLOCK_LENGTH: usize = 1024 * 16; // 16kB
const DECRYPTION_BLOCK_LENGTH: usize = ENCRYPTION_BLOCK_LENGTH + 16;

pub struct FileDecryptionStream {}

impl FileEncryptionStream {
    pub fn from_file(
        file: File,
        master_key: MasterKey,
        stream_id: StreamId,
    ) -> FileEncryptionStream {
        let file_key = master_key.file_key();
        let nonce = master_key.aead_nonce();

        let file_encryption_key = file_key.get_file_encryption_key(stream_id);
        let aead = Aead::new(file_encryption_key.deref().into());

        let aead_stream = AeadEncryptor::from_aead(aead, nonce.as_ref().into());

        Self {
            reader: file,
            aead_stream,
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
                    dbg!(remaining);
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

        println!("Encrypted {bytes_encrypted} bytes");

        Ok(())
    }
}

impl FileDecryptionStream {
    // TODO: Take in a `File` and slowly write to it instead of returning bytes
    pub async fn decrypt(
        ticket: Ticket,
        stream_id: StreamId,
        mut stream: RecvStream,
    ) -> Result<(FlapFileMetadata, Bytes)> {
        let master_key = ticket.master_key();
        let file_key = master_key.file_key();

        let file_encryption_key = file_key.get_file_encryption_key(stream_id);

        let file_metadata_info_length = stream
            .read_u64()
            .await
            .map_err(|_| crate::error::Error::FileReadError)?;

        let mut file_metadata_bytes = BytesMut::zeroed(file_metadata_info_length as usize);
        stream.read_exact(&mut file_metadata_bytes).await.unwrap();
        let file_metadata = FlapFileMetadata::from_bytes(file_metadata_bytes.into()).await;

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
                        decrypted_bytes += DECRYPTION_BLOCK_LENGTH;
                    }
                }
                Ok(None) => {
                    // Stream is complete and we don't have a full block's amount of bytes
                    // We can therefore finish decryption
                    file_plaintext_bytes.put(aead_stream.decrypt_last(buffer.as_ref())?.as_slice());
                    decrypted_bytes += buffer.len();
                    dbg!(decrypted_bytes);
                    break;
                }
                Err(_) => panic!("decryption stream failed"),
            }

            // Returned None
        }

        Ok((file_metadata, file_plaintext_bytes.into()))
    }
}
