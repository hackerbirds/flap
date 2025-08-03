//! Info needed to connect to a device
//! Right now only includes the iroh endpoint of device

use std::str::FromStr;

use iroh::NodeAddr;
use iroh_blobs::ticket::BlobTicket;

use crate::{crypto::master_key::MasterKey, error::Error};

pub struct Ticket {
    pub blob_ticket: BlobTicket,
    master_key: MasterKey,
}

impl Ticket {
    pub fn master_key(&self) -> &MasterKey {
        &self.master_key
    }

    pub fn make(blob_ticket: BlobTicket, master_key: MasterKey) -> Self {
        Self {
            blob_ticket,
            master_key,
        }
    }

    pub fn convert(&self) -> String {
        format!(
            "flap/{}/{}",
            self.blob_ticket,
            self.master_key.encode_to_string()
        )
    }
}

impl Into<NodeAddr> for Ticket {
    fn into(self) -> NodeAddr {
        self.blob_ticket.node_addr().clone()
    }
}

impl FromStr for Ticket {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ticket_string = s.to_owned();
        let split: Vec<&str> = ticket_string.split('/').collect();
        let flap = split.get(0).ok_or(Error::TicketParseError)?;
        let blob_ticket_str = split.get(1).ok_or(Error::TicketParseError)?;
        let master_key_str = split.get(2).ok_or(Error::TicketParseError)?;

        if flap != &"flap" {
            return Err(Error::TicketParseError);
        }

        let blob_ticket = blob_ticket_str
            .parse()
            .map_err(|_| Error::TicketParseError)?;
        let master_key = master_key_str.parse()?;

        Ok(Self {
            blob_ticket,
            master_key,
        })
    }
}
