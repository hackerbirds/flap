use std::ops::Deref;

use crate::error::Result;

#[derive(Debug)]
pub struct P2pEndpoint(iroh::Endpoint);

impl P2pEndpoint {
    pub async fn start() -> Result<Self> {
        let endpoint = iroh::Endpoint::builder()
            .discovery_n0()
            .bind()
            .await?;

        Ok(Self(endpoint))
    }

    pub fn to_iroh_endpoint(self) -> iroh::Endpoint {
        self.0
    }
}

impl Deref for P2pEndpoint {
    type Target = iroh::Endpoint;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
