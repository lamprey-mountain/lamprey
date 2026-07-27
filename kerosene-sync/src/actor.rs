use kerosene_core::compat::authz::AuthCheck;
use lamprey::v1::types::Session;
use lamprey::v2::types::{ConnectionId, SessionId};
use tokio::task::JoinHandle;

use crate::prelude::*;
use crate::transport::AnyTransport;

// TODO: impl Debug
pub struct ConnectionActor {
    // TODO
}

// TODO: impl Debug
#[derive(Clone)]
pub struct ConnectionHandle {
    id: ConnectionId,
    // tx: mpsc::Sender<Command>,
}

impl ConnectionActor {
    // pub fn new(transport: AnyTransport) -> Self {
    //     todo!()
    // }

    pub(crate) fn create(session: Session /* presence, transport, etc */) -> ConnectionHandle {
        todo!()
    }

    pub fn handle(&self) -> ConnectionHandle {
        todo!()
    }

    pub fn spawn(mut self) -> JoinHandle<()> {
        todo!()
    }
}

impl ConnectionHandle {
    pub fn id(&self) -> ConnectionId {
        self.id
    }

    pub fn session_id(&self, session: &Session) -> SessionId {
        session.id
    }

    /// attach a transport to this connection and rewind
    pub fn attach(&self, transport: AnyTransport, seq: u64) {
        // let _ = self.tx.try_send(Command::Attach(transport, seq));
        todo!()
    }

    /// shutdown this connection
    pub fn shutdown(&self) {
        // TODO: use CancellationToken instead
        // let _ = self.tx.try_send(Command::Shutdown);
        todo!()
    }

    // /// stream events from this connection?
    // pub fn events(&self) { todo!() }
}

pub trait Driver {
    async fn check_permission(&self, check: &AuthCheck) -> bool;
    // pub async fn fetch_ready(&self) -> ();
}
