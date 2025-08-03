use std::ops::Deref;

use hkdf::Hkdf;
use sha2::Sha256;

use crate::crypto::master_key::MasterKey;

pub struct FileKey([u8; 32]);

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

impl Deref for FileKey {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
