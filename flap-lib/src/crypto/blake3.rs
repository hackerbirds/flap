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
}
