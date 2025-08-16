use bytes::BytesMut;
use tokio::{fs::File, io::AsyncReadExt};

use crate::error::Result;

pub type FileHash = [u8; 32];

/// A `blake3` hasher, used to verify that the final
/// file is valid after a transfer's completion.
///
/// Even though the AEAD guarantees that the bytes
/// received are what the sender intended, there
/// is no proven guarantee as to whether the file
/// was sent in order, or bytes were skipped, etc.
///
/// This is why this check is done.
#[derive(Debug, Default)]
pub struct Blake3(blake3::Hasher);

impl Blake3 {
    pub fn update_hasher(&mut self, data: &[u8]) {
        self.0.update(data);
    }

    pub fn finalize_hash(&mut self) -> FileHash {
        self.0.finalize().into()
    }

    /// Reads entire file given and returns the hasher in its current state.
    ///
    /// Used for resuming a partially completed transfer, since we still want
    /// to verify the hash of the entire file.
    ///
    /// Reads up to `max` bytes. In our case, `max` is the file length of the
    /// partial file the receiver already has. The sender has the full file, so
    /// they want to read only up to the portion the receiver has to get the partial
    /// hash.
    pub async fn partial_hash(file: &mut File, max: Option<u64>) -> Result<Self> {
        let mut hasher = Self::default();
        const BUF_SIZE: usize = 1 << 16;
        let mut total_read = 0;
        let mut file_buf = BytesMut::zeroed(BUF_SIZE);
        let max_seek = max.unwrap_or(file.metadata().await?.len()) as usize;
        loop {
            match file.read(&mut file_buf).await? {
                0 => break,
                len => {
                    if max_seek > 0 && max_seek - total_read < len {
                        hasher.update_hasher(&file_buf[0..max_seek - total_read]);
                        break;
                    } else {
                        hasher.update_hasher(&file_buf[0..len]);
                        total_read += len;
                    }
                }
            }
        }

        Ok(hasher)
    }
}
