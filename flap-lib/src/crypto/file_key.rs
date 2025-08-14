use hkdf::Hkdf;
use sha2::Sha256;

use crate::crypto::master_key::MasterKey;

/// This key is used for all files within one connection.
pub struct FileKey([u8; 32]);

impl FileKey {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
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
