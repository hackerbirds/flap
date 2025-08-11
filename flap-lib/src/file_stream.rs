use aead::KeyInit;
use bytes::{BufMut, BytesMut};
use iroh::endpoint::{RecvStream, SendStream};
use std::mem::MaybeUninit;
use std::ops::Deref;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::crypto::master_key::MasterKey;
use crate::crypto::transfer_id::TransferId;
use crate::error::Result;
use crate::event::{Event, get_event_handler};
use crate::fs::metadata::{FlapFileMetadata, MAX_METADATA_LENGTH_ALLOWED};
use crate::fs::save::FileSaver;
use crate::ticket::Ticket;

type Aead = chacha20poly1305::XChaCha20Poly1305;
type AeadEncryptor = aead::stream::EncryptorBE32<Aead>;
type AeadDecryptor = aead::stream::DecryptorBE32<Aead>;

pub struct FileEncryptor {
    file: File,
    aead_stream: AeadEncryptor,
    transfer_id: TransferId,
}

const ENCRYPTION_BLOCK_LENGTH: usize = 1 << 16; // 64kB
const DECRYPTION_BLOCK_LENGTH: usize = ENCRYPTION_BLOCK_LENGTH + 16;

impl FileEncryptor {
    pub fn from_file(file: File, master_key: MasterKey, transfer_id: TransferId) -> FileEncryptor {
        let file_key = master_key.file_key();
        let nonce = master_key.aead_nonce();

        let file_encryption_key = file_key.get_file_encryption_key(transfer_id);
        let aead = Aead::new(file_encryption_key.deref().into());

        let aead_stream = AeadEncryptor::from_aead(aead, nonce.as_ref().into());

        Self {
            file,
            aead_stream,
            transfer_id,
        }
    }

    pub async fn encrypt(
        mut self,
        metadata: FlapFileMetadata,
        stream: &mut SendStream,
    ) -> Result<()> {
        let mut bytes_encrypted = 0;
        let mut buf = BytesMut::new();

        let metadata_bytes = metadata.to_bytes();
        let metadata_length = metadata_bytes.len();

        buf.put_u64(metadata_length as u64);
        buf.put_slice(&metadata_bytes);

        loop {
            match self.file.read_buf(&mut buf).await {
                // Assumption: files in filesystems only have one `EOF`
                // and it's at the end of the, well, file. Therefore,
                // decryption must finish.
                Ok(0) => {
                    stream
                        .write_all(self.aead_stream.encrypt_last(buf.as_ref())?.as_slice())
                        .await
                        .map_err(|_| crate::error::Error::FileReadError)?;
                    break;
                }
                Ok(_bytes_read) => {
                    if buf.len() >= ENCRYPTION_BLOCK_LENGTH {
                        let block = buf.split_to(ENCRYPTION_BLOCK_LENGTH);

                        stream
                            .write_all(self.aead_stream.encrypt_next(block.as_ref())?.as_slice())
                            .await
                            .map_err(|_| crate::error::Error::FileReadError)?;

                        bytes_encrypted += ENCRYPTION_BLOCK_LENGTH;

                        get_event_handler().send_event(Event::TransferUpdate(
                            self.transfer_id,
                            bytes_encrypted as u64,
                        ));
                    }
                }
                Err(_) => panic!("Encryption failed"),
            }
        }

        get_event_handler().send_event(Event::TransferComplete(self.transfer_id));

        Ok(())
    }
}

pub struct FileDecryptor {}

impl FileDecryptor {
    async fn write_with_metadata(
        file_saver: &FileSaver,
        file_transfer_id: TransferId,
        output_file: &mut MaybeUninit<File>,
        mut decrypted_block: Vec<u8>,
        decrypted_bytes: &mut u64,
    ) {
        let output_file_init = if 0.eq(decrypted_bytes) {
            let mut metadata_length = decrypted_block;

            let mut metadata_bytes = metadata_length.split_off(8 as usize);

            let metadata_size = u64::from_be_bytes(
                metadata_length
                    .as_slice()
                    .try_into()
                    .expect("vec length is exactly 8"),
            );

            assert!(metadata_size < MAX_METADATA_LENGTH_ALLOWED);

            decrypted_block = metadata_bytes.split_off(metadata_size as usize);

            let metadata = FlapFileMetadata::from_bytes(metadata_bytes.into()).await;

            let file = file_saver.prepare_file(&metadata.file_name).await.unwrap();

            get_event_handler().send_event(Event::PreparingFile(file_transfer_id, metadata, false));

            let _ = std::mem::replace(decrypted_bytes, *decrypted_bytes + 8 + metadata_size);

            output_file.write(file)
        } else {
            // SAFETY: `decrypted_bytes` is initialized at 0, so the `if` condition
            // above setting up `output_file` is guaranteed to be executed at least once
            unsafe { output_file.assume_init_mut() }
        };

        output_file_init
            .write_all(decrypted_block.as_slice())
            .await
            .expect("file can be written to");
    }

    // TODO: Take in a `File` and slowly write to it instead of returning bytes
    pub async fn launch(
        ticket: Ticket,
        file_transfer_id: TransferId,
        mut receive_stream: RecvStream,
        file_saver: FileSaver,
    ) -> Result<()> {
        let master_key = ticket.master_key();
        let file_key = master_key.file_key();

        let file_encryption_key = file_key.get_file_encryption_key(file_transfer_id);

        let nonce = master_key.aead_nonce();
        let aead = Aead::new(file_encryption_key.deref().into());

        let mut aead_stream = AeadDecryptor::from_aead(aead, nonce.as_ref().into());

        let mut buffer = BytesMut::new();
        let mut decrypted_bytes = 0;

        let mut output_file: MaybeUninit<File> = MaybeUninit::uninit();

        loop {
            // TODO: Support for unordered/parallel AEAD would allow for faster file transfer
            match receive_stream
                .read_chunk(DECRYPTION_BLOCK_LENGTH, true)
                .await
            {
                Ok(Some(chunk)) => {
                    buffer.put(chunk.bytes);
                    if buffer.len() >= DECRYPTION_BLOCK_LENGTH {
                        let block = buffer.split_to(DECRYPTION_BLOCK_LENGTH);

                        let plaintext_block = aead_stream.decrypt_next(block.as_ref())?;

                        FileDecryptor::write_with_metadata(
                            &file_saver,
                            file_transfer_id,
                            &mut output_file,
                            plaintext_block,
                            &mut decrypted_bytes,
                        )
                        .await;

                        get_event_handler().send_event(Event::TransferUpdate(
                            file_transfer_id,
                            decrypted_bytes as u64,
                        ));

                        decrypted_bytes += DECRYPTION_BLOCK_LENGTH as u64;
                    }
                }
                Ok(None) => {
                    // Stream is complete and we don't have a full block's amount of bytes
                    // We can therefore finish decryption
                    let last_plaintext_block = aead_stream.decrypt_last(buffer.as_ref())?;

                    FileDecryptor::write_with_metadata(
                        &file_saver,
                        file_transfer_id,
                        &mut output_file,
                        last_plaintext_block,
                        &mut decrypted_bytes,
                    )
                    .await;
                    break;
                }
                Err(_) => panic!("decryption stream failed"),
            }
        }

        get_event_handler().send_event(Event::TransferComplete(file_transfer_id));

        Ok(())
    }
}
