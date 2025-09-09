use std::ops::Deref;

#[cfg(feature = "tracing")]
use tracing::warn;

use crate::{error::Result, p2p::ALPN};

#[derive(Debug, Clone)]
pub struct P2pEndpoint(iroh::Endpoint);

impl P2pEndpoint {
    pub async fn start() -> Result<Self> {
        let endpoint = iroh::Endpoint::builder()
            .alpns(vec![ALPN.to_vec()])
            .discovery_n0()
            .bind()
            .await?;

        Ok(Self(endpoint))
    }
}

impl Deref for P2pEndpoint {
    type Target = iroh::Endpoint;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for P2pEndpoint {
    fn drop(&mut self) {
        #[cfg(feature = "tracing")]
        warn!("dropping endpoint!")
    }
}
