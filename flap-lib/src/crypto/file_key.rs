use std::ops::Deref;

use hkdf::Hkdf;
use sha2::Sha256;

use crate::{crypto::master_key::MasterKey, file_stream::TransferId};

/// This key is used for all files within one connection.
pub struct FileKey([u8; 32]);

/// The encryption key used *per file*. It is derived from [`FileKey`]
/// and per [`TransferId`], since there is one QUIC stream per file, and that
/// QUIC requires [`TransferId`] to be unique and not reused.
pub struct FileEncryptionKey([u8; 32]);

impl FileKey {
    pub fn get_file_encryption_key(&self, file_transfer_id: TransferId) -> FileEncryptionKey {
        let ikm = self.0.as_slice();
        let hk = Hkdf::<Sha256>::new(None, &ikm);
        let mut file_encryption_key = [0u8; 32];

        let info = file_transfer_id.as_ref();
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
