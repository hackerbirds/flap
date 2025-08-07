use std::ops::Deref;

use hkdf::Hkdf;
use iroh::endpoint::{StreamId, VarInt};
use sha2::Sha256;

use crate::crypto::master_key::MasterKey;

pub struct FileKey([u8; 32]);

/// The encryption key used per file. It is derived from [`FileKey`]
/// and per [`StreamId`], since there is one QUIC stream per file, and that
/// QUIC requires [`StreamId`] to be unique and not reused.
pub struct FileEncryptionKey([u8; 32]);

impl FileKey {
    pub fn get_file_encryption_key(&self, stream_id: StreamId) -> FileEncryptionKey {
        let ikm = self.0.as_slice();
        let hk = Hkdf::<Sha256>::new(None, &ikm);
        let mut file_encryption_key = [0u8; 32];

        let stream_id_u64 = VarInt::from(stream_id).into_inner();
        dbg!(stream_id_u64);
        let info = stream_id_u64.to_le_bytes();
        hk.expand(&info, &mut file_encryption_key)
            .expect("valid length output");

        FileEncryptionKey(file_encryption_key)
    }
}

impl MasterKey {
    pub fn file_key(&self) -> FileKey {
        const HKDF_INFO: &[u8] = b"flap_file_key";

        let ikm = self.0.as_slice();

        let hk = Hkdf::<Sha256>::new(None, &ikm);
        let mut file_key = [0u8; 32];
        hk.expand(&HKDF_INFO, &mut file_key)
            .expect("valid length output");

        FileKey(file_key)
    }
}

impl Deref for FileEncryptionKey {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
