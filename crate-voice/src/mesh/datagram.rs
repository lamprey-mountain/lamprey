//! wire format for datagrams

use crate::prelude::*;
use common::v1::types::voice::datachannel::SpeakingDatagram;

#[derive(Debug)]
pub enum Datagram {
    Speaking(SpeakingDatagram),
}

impl Datagram {
    pub fn parse(data: &[u8]) -> Result<Self> {
        todo!()
    }
}
