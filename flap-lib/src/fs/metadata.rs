use std::path::PathBuf;

use bytes::{Buf, BufMut, Bytes, BytesMut};
use tokio::fs::metadata;

pub const MAX_METADATA_LENGTH_ALLOWED: u64 = 1 << 13; // 8kB max

/// Basic file metadata structure.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct FlapFileMetadata {
    // TODO: We will allow dirs in the duture
    is_file: bool,
    // If this is a directory, list files inside directory
    // We only support files for now
    dir_file_entries: Option<Vec<FlapFileMetadata>>,
    pub file_size: u64,
    pub file_name: String,
}

impl FlapFileMetadata {
    pub async fn from_path(file_path: &PathBuf) -> Self {
        let metadata = metadata(file_path).await.unwrap();

        Self {
            is_file: metadata.is_file(),
            dir_file_entries: None,
            // TODO: bruh
            file_name: file_path
                .file_name()
                .unwrap()
                .to_os_string()
                .into_string()
                .unwrap(),
            file_size: metadata.len(),
        }
    }

    pub async fn from_bytes(mut bytes: Bytes) -> Self {
        let file_size = bytes.get_u64();
        let file_name = String::from_utf8(bytes.to_vec()).unwrap();

        Self {
            is_file: true,
            dir_file_entries: None,
            file_size,
            file_name,
        }
    }

    pub fn to_bytes(self) -> Bytes {
        let mut metadata_bytes = BytesMut::new();
        metadata_bytes.put_u64(self.file_size);
        metadata_bytes.put_slice(self.file_name.as_bytes());

        metadata_bytes.into()
    }
}
