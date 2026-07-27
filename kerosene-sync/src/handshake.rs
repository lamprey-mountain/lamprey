use futures::StreamExt;
use lamprey::v1::types::{MessageClient, SyncParams};

use crate::prelude::*;
use crate::{
    actor::ConnectionActor,
    transport::{AnyTransport, TransportEvent},
};

/// utility to accept new sync connections and do handshakes on them
///
/// ie. wait for `Hello`
pub struct Handshake {
    transport: AnyTransport,
}

impl Handshake {
    pub fn new(transport: AnyTransport, params: SyncParams) -> Self {
        Self { transport }
    }

    pub async fn finish(self) -> Result<ConnectionActor> {
        let (_sink, mut stream) = self.transport.split();

        match stream.next().await {
            Some(Ok(TransportEvent::Message(MessageClient::Hello(hello)))) => {
                // TODO: impl
                Ok(todo!())
            }
            // TODO: better errors
            Some(Ok(_)) => Err(Error::BadStatic("expected Hello message")),
            Some(Err(e)) => Err(e),
            None => Err(Error::BadStatic("transport closed before Hello")),
        }
    }
}
