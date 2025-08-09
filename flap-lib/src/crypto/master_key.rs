use std::{fmt::Debug, str::FromStr};

use base64ct::{Base64Url, Encoding};

use crate::{crypto::random_array, error::Error};

#[derive(Clone, Copy)]
pub struct MasterKey(pub(crate) [u8; 16]);

impl Debug for MasterKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("MasterKey").field(&"[REDACTED]").finish()
    }
}

impl MasterKey {
    pub fn generate() -> Self {
        let key = random_array::<16>();
        Self(key)
    }

    pub fn encode_to_string(&self) -> String {
        Base64Url::encode_string(&self.0)
    }
}

impl FromStr for MasterKey {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match Base64Url::decode_vec(s) {
            Ok(bytes) => {
                let array = bytes.try_into().map_err(|_| Error::MasterKeyParseError)?;

                Ok(MasterKey(array))
            }
            Err(_) => Err(Error::MasterKeyParseError),
        }
    }
}
