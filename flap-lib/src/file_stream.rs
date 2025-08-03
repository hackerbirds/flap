use aead::rand_core::RngCore;
use aead::{KeyInit, OsRng};
use bytes::{BufMut, Bytes, BytesMut};
use iroh_blobs::api::blobs::BlobReader;
use std::fs::File;
use std::io::{Error, ErrorKind, Read};
use std::ops::Deref;
use std::task::Poll;
use tokio::io::AsyncReadExt;
use tokio_stream::Stream;

use crate::crypto::master_key::MasterKey;
use crate::error::Result;

type Aead = chacha20poly1305::XChaCha20Poly1305;
type AeadEncryptor = aead::stream::EncryptorBE32<Aead>;
type AeadDecryptor = aead::stream::DecryptorBE32<Aead>;
const BUFFER_LENGTH: usize = 1 << 16;

pub struct FileEncryptionStream {
    reader: File,
    aead_stream: AeadEncryptor,
    buffer: BytesMut,
}

pub struct FileDecryptionStream {
    // for now the whole file is stored in memory which doesn't make sense and in the future
    // we'll also have fs-based streaming
    blob_reader: BlobReader,
    aead_stream: AeadDecryptor,
}

impl FileEncryptionStream {
    pub fn from_file(file: File, master_key: MasterKey) -> FileEncryptionStream {
        let file_key = master_key.file_key();
        let nonce = master_key.aead_nonce();
        let aead = Aead::new(file_key.deref().into());

        let aead_stream = AeadEncryptor::from_aead(aead, nonce.as_ref().into());

        let buffer = BytesMut::zeroed(BUFFER_LENGTH);

        Self {
            reader: file,
            aead_stream,
            buffer,
        }
    }
}

impl FileDecryptionStream {
    pub fn new(blob_reader: BlobReader, master_key: &MasterKey) -> Self {
        let file_key = master_key.file_key();
        let nonce = master_key.aead_nonce();
        let aead = Aead::new(file_key.deref().into());

        let aead_stream = AeadDecryptor::from_aead(aead, nonce.as_ref().into());

        Self {
            blob_reader,
            aead_stream,
        }
    }

    // TODO: turn into stream impl?
    pub async fn decrypt(mut self) -> Result<Bytes> {
        let mut file_bytes = BytesMut::new();
        const BUFFER_LENGTH: usize = 1024 + 16;
        let mut buf = [0u8; BUFFER_LENGTH];

        loop {
            match self.blob_reader.read(&mut buf).await {
                Ok(BUFFER_LENGTH) => {
                    let plaintext = self.aead_stream.decrypt_next(buf.as_slice())?;
                    file_bytes.put(plaintext.as_slice());
                }
                Ok(remaining) => {
                    let plaintext = self.aead_stream.decrypt_last(&buf[..remaining])?;
                    file_bytes.put(plaintext.as_slice());
                    break;
                }
                Err(_) => panic!("decryption failed"),
            }
        }

        Ok(file_bytes.into())
    }
}

impl Stream for FileEncryptionStream {
    type Item = core::prelude::v1::Result<Bytes, std::io::Error>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        // read next BUFFER_LENGTH bytes
        let mut buf = self.buffer.clone();

        match self.reader.read(buf.as_mut()) {
            Ok(BUFFER_LENGTH) => Poll::Ready(Some(
                self.aead_stream
                    .encrypt_next(&*buf)
                    .map(|vec| Bytes::from(vec))
                    .map_err(|aead_err| Error::new(ErrorKind::InvalidInput, aead_err)),
            )),
            Ok(remaining) => {
                if remaining == 0 {
                    Poll::Ready(None)
                } else {
                    // `encrypt_last` consumes the AEAD object for safety
                    // but we only have a &mut here. We use `std::mem::replace` as a trick
                    // to own the AEAD
                    let mut dummy_nonce = [0u8; 19];
                    OsRng.fill_bytes(&mut dummy_nonce);
                    let dummy_aead_key = Aead::generate_key(OsRng);
                    let dummy_aead = Aead::new(&dummy_aead_key);
                    let dummy_aead_stream =
                        AeadEncryptor::from_aead(dummy_aead, dummy_nonce.as_ref().into());

                    let aead_stream = std::mem::replace(&mut self.aead_stream, dummy_aead_stream);

                    let bytes = aead_stream
                        .encrypt_last(&buf[..remaining])
                        .map(|vec| Bytes::from(vec))
                        .map_err(|aead_err| Error::new(ErrorKind::InvalidInput, aead_err));

                    Poll::Ready(Some(bytes))
                }
            }
            Err(_) => Poll::Ready(None),
        }
    }
}
