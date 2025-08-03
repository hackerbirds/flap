use std::ops::Deref;

use hkdf::Hkdf;
use sha2::Sha256;

use crate::crypto::master_key::MasterKey;

/// A nonce for use in the AEAD stream cipher.
/// 
/// This should only be used once, but I will design
/// the app in a way that the Ticket is a one-time code.
pub struct AeadStreamNonce([u8; 19]);

impl MasterKey {
    pub fn aead_nonce(&self) -> AeadStreamNonce {
        const HKDF_INFO: &[u8] = b"flap_aead_stream_nonce";

        let ikm = self.0.as_slice();

        let hk = Hkdf::<Sha256>::new(None, &ikm);
        let mut nonce = [0u8; 19];
        hk.expand(&HKDF_INFO, &mut nonce)
            .expect("valid length output");

        AeadStreamNonce(nonce)
    }
}

impl Deref for AeadStreamNonce {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
