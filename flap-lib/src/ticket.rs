//! Info needed to connect to a device
//! Right now only includes the iroh endpoint of device

use std::str::FromStr;

use iroh::NodeId;

use crate::{crypto::master_key::MasterKey, error::Error};

#[derive(Debug, Clone)]
pub struct Ticket {
    pub node_id: NodeId,
    master_key: MasterKey,
}

impl Ticket {
    pub fn master_key(&self) -> &MasterKey {
        &self.master_key
    }

    pub fn make(node_id: NodeId, master_key: MasterKey) -> Self {
        Self {
            node_id,
            master_key,
        }
    }

    pub fn convert(&self) -> String {
        format!(
            "flap/{}/{}",
            self.node_id,
            self.master_key.encode_to_string()
        )
    }
}

impl FromStr for Ticket {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ticket_string = s.to_owned();
        let split: Vec<&str> = ticket_string.split('/').collect();
        let flap = split.get(0).ok_or(Error::TicketParseError)?;
        let node_id_str = split.get(1).ok_or(Error::TicketParseError)?;
        let master_key_str = split.get(2).ok_or(Error::TicketParseError)?;

        if flap != &"flap" {
            return Err(Error::TicketParseError);
        }

        let node_id = node_id_str.parse().map_err(|_| Error::TicketParseError)?;
        let master_key = master_key_str.parse()?;

        Ok(Self {
            node_id,
            master_key,
        })
    }
}
