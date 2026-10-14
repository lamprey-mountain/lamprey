use futures::StreamExt;
use kerosene_core::prelude::*;
use lamprey::v1::types::error::SyncErrorCode;
use lamprey::v1::types::{MessageClient, SyncParams};

use crate::prelude::*;
use crate::{
    actor::ConnectionActor,
    transport::{AnyTransport, TransportEvent},
};

/// utility to accept new sync connections and do handshakes on them
///
/// ie. wait for `Hello`
pub struct Handshake<T> {
    transport: T,
}

impl<T: Transport> Handshake<T> {
    pub fn new(transport: T, params: SyncParams) -> Self {
        Self { transport }
    }

    // pub async fn finish(self) -> Result<ConnectionActor> {
    pub async fn finish(self) -> ServerResult<()> {
        let (mut send, mut recv) = self.transport.split();

        // NOTE: do i need to set up a minimal impl of the protocol here?
        let init = tokio::time::timeout(Duration::from_secs(5), recv.next()).await;

        // outer result: tokio timeout
        // option: client not sending any more messages
        // inner result: transport errors
        match init {
            Ok(Some(Ok(TransportEvent::Message(init)))) => {
                // TODO: impl
                Ok(todo!())
            }

            // TODO: better errors
            Ok(Some(Ok(_))) => Err(Error::BadStatic("expected Hello message")),
            Ok(Some(Err(e))) => Err(e),
            Ok(None) => Err(Error::BadStatic("transport closed before Hello")),
            Err(e) => Err(Error::BadStatic("transport timed out ")),
        }

        // SyncErrorCode::Timeout;
        // SyncErrorCode::Unauthorized;
    }
}
