use std::{fmt::Debug, str::FromStr};

use crate::{crypto::random_array, error::Error};

#[derive(Clone, Copy)]
pub struct MasterKey(pub(crate) [u8; 32]);

impl Debug for MasterKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("MasterKey").field(&"[REDACTED]").finish()
    }
}

impl MasterKey {
    pub fn generate() -> Self {
        let key = random_array::<32>();
        Self(key)
    }

    pub fn encode_to_string(&self) -> String {
        hex::encode(self.0)
    }
}

impl FromStr for MasterKey {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match hex::decode(s) {
            Ok(bytes) => {
                let array = bytes.try_into().map_err(|_| Error::MasterKeyParseError)?;

                Ok(MasterKey(array))
            }
            Err(_) => Err(Error::MasterKeyParseError),
        }
    }
}
