use std::time::Duration;

use crate::{backbone::datagram::Datagram, prelude::*};

/// manages a connection to another sfu
pub struct Remote {
    // TODO
}

pub struct RemoteHandle {
    // TODO
}

impl Remote {
    // #[tracing::instrument(skip(self, incoming))]
    // pub async fn handle_incoming(&mut self, incoming: quinn::Incoming) {
    //     todo!()
    // }
}

impl RemoteHandle {
    /// get round trip time
    pub fn rtt(&self) -> Option<Duration> {
        todo!()
    }

    /// subscribe to a track
    pub async fn subscribe(&self /* ... */) -> Result<()> {
        todo!()
    }

    pub async fn send_datagram(&self, datagram: Datagram) -> Result<()> {
        todo!()
    }

    pub async fn probe(&self) -> Result<()> {
        todo!()
    }

    pub async fn goodbye(&self) -> Result<()> {
        todo!()
    }
}
