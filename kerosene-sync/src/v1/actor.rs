use std::collections::HashMap;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::stream::FuturesUnordered;
use futures::{Stream, StreamExt};
use kerosene_core::compat::authz::AuthCheck;
use lamprey::v1::types::Session;
use lamprey::v2::types::{ConnectionId, SessionId};
use tokio::task::JoinHandle;

use crate::prelude::*;
use crate::transport::{AnyTransport, TransportSink, TransportStream};

/// handles multiple connections at once
pub struct ConnectionPool {
    globals: Globals,
    connections: DashMap<ConnectionId, ConnectionActor>,
    rx: mpsc::Receiver<Command>,
    // TODO: finish struct
    // maybe add:
    // streams: (),
    // subscriptions: ()
    // tasks: JoinSet<()>,
}

pub enum Command {}

pub enum Event {}

pub struct StreamMap<K, V> {
    // PERF: use dashmap
    // streams: DashMap<K, V>,
    streams: HashMap<K, V>,
}

impl<K: Hash + Eq, V> StreamMap<K, V> {
    pub fn insert(&mut self, key: K, stream: V) {
        self.streams.insert(key, stream);
    }
}

impl<K, V: Stream> Stream for StreamMap<K, V> {
    type Item = V::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        for s in &mut self.streams {}

        Poll::Pending;
        todo!()
    }
}

impl ConnectionPool {
    fn a(&self) {
        let mut stream: TransportStream;
        let mut sink: TransportSink;
        futures::future::join_all(iter);
        futures::stream::select_all(streams)
    }
}

pub struct ConnectionStream {
    queue: ConnectionQueue,
    transport: Option<ConnectionTransport>,
}

pub struct ConnectionTransport {
    send: Box<dyn TransportSink>,
    recv: TransportStream,
    timeout: Timeout,
}

// TODO: impl Debug
pub struct ConnectionActor {
    id: ConnectionId,
    session: Session, // TODO: replace with minimal session object
    subscriptions: Box<ConnectionSubscriptions>,
    streams: HashMap<StreamSubscription, ConnectionStream>,
    rx: mpsc::Receiver<Command>, // TODO: rename field?
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
