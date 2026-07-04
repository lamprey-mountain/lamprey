//! work in progress redesign of the sync system

use crate::sync::connection_queue::ConnectionQueue;
use crate::sync::subscriptions::ConnectionSubscriptions;
use crate::sync::transport::{
    AnyTransport, Transport, TransportEvent, TransportSink, TransportStream, WebsocketTransport,
};
use crate::sync::util::Timeout;
use crate::sync::util::{ConnectionState, MAX_QUEUE_LEN};
use crate::{ServerState, prelude::*};
use common::v1::types::{
    MessageClient, MessageEnvelope, MessagePayload, MessageSync, Session, SyncParams,
};
use common::v2::types::{ConnectionId, SessionId};
use futures_util::StreamExt;
use tokio::sync::mpsc;
use tracing::{Instrument, debug, error, trace, warn};

/// an authenticated connection
pub struct Connection2 {
    id: ConnectionId,
    session: Session,
    queue: ConnectionQueue,
    subscriptions: Box<ConnectionSubscriptions>,
    transport: Option<ConnectionTransport>,
    globals: Globals,
    rx: mpsc::Receiver<Command>,
}

pub struct ConnectionTransport {
    send: Box<dyn TransportSink>,
    recv: TransportStream,
    timeout: Timeout,
}

#[derive(Clone)]
pub struct ConnectionHandle {
    tx: mpsc::Sender<Command>,
    id: ConnectionId,
}

// TODO: rename to ConnectionCommand
/// a command for controlling a connection actor
pub enum Command {
    /// attach a transport to this connection
    Attach(Box<dyn Transport>),

    /// shutdown this connection
    Shutdown,
}

// TODO: rename to ConnectionEvent
/// an event emitted by a connection actor
pub enum Event {
    /// this connection's transport was detached
    Detached,
}

impl Connection2 {
    pub fn create(globals: Globals, session: Session) -> ConnectionHandle {
        let id = ConnectionId::new();
        let queue = ConnectionQueue::new(MAX_QUEUE_LEN);
        let subscriptions = Box::new(ConnectionSubscriptions::new(globals.clone(), id));
        let (tx, rx) = mpsc::channel(16);

        let me = Self {
            id,
            session,
            queue,
            subscriptions,
            transport: None,
            globals,
            rx,
        };

        let handle = ConnectionHandle { tx, id };

        tokio::spawn(
            async move {
                me.spawn().await;
            }
            .instrument(tracing::debug_span!("connection", id = %id)),
        );

        handle
    }

    async fn spawn(mut self) {
        let mut sushi = self.globals.messaging().subscribe().await.unwrap();

        loop {
            // transport_futures event
            enum Tfe {
                Recv(Option<Result<TransportEvent>>),
                Timeout,
            }

            let transport_futures = async {
                if let Some(t) = &mut self.transport {
                    tokio::select! {
                        event = t.recv.next() => Tfe::Recv(event),
                        _ = tokio::time::sleep_until(t.timeout.get_instant()) => Tfe::Timeout,
                    }
                } else {
                    futures_util::future::pending().await
                }
            };

            tokio::select! {
                // poll transports
                event = transport_futures => {
                    match event {
                        Tfe::Recv(Some(Ok(event))) => {
                            if let Err(err) = self.handle_client(event).await {
                                error!("handle_client error: {err}");
                                // TODO: don't break on any error
                                break;
                            }
                        }
                        Tfe::Recv(Some(Err(_err))) => {
                            // TODO: handle Err
                        }
                        Tfe::Recv(None) => {
                            // TODO: handle None (transport closed)
                        }
                        Tfe::Timeout => {
                            if let Err(err) = self.handle_timeout().await {
                                error!("handle_timeout error: {err}");
                                break;
                            }
                            // TODO: handle Timeout::Close
                        }
                    }
                }

                // poll sushi
                Some(msg) = sushi.next() => {
                    // TODO: queue sushi message
                    // NOTE: the commented out code below won't work, msg is a Broadcast not MessageSync
                    // if let Some(t) = &mut self.transport {
                    //     if let Err(err) = self.queue_message(Box::new(msg.message), msg.nonce, &mut t.send).await {
                    //         debug!("failed to queue sushi message: {err}");
                    //     }
                    // }
                }

                // poll subscriptions
                sub_res = self.subscriptions.poll() => {
                    match sub_res {
                        Ok(msg) => {
                            // TODO: queue subscription message
                            // if let Some(t) = &mut self.transport {
                            //     if let Err(err) = self.queue_message(Box::new(msg), None, &mut t.send).await {
                            //         error!("failed to queue subscription message: {err}");
                            //     }
                            // }
                        }
                        Err(err) => {
                            error!("subscription poll error: {err}");
                             // TODO: don't break on any error
                            break;
                        }
                    }
                }

                // handle commands
                Some(cmd) = self.rx.recv() => {
                    if let Err(err) = self.handle_command(cmd).await {
                        error!("handle_command error: {err}");
                        break;
                    }
                }
            }
        }
    }

    /// handle an event from the client
    async fn handle_client(&mut self, event: TransportEvent) -> Result<()> {
        match event {
            TransportEvent::Message(msg) => {
                let Some(t) = &mut self.transport else {
                    unreachable!("how did we receive a client event without an active transport?")
                };

                // TODO
                // self.conn
                //     .handle_message_client(msg, &mut *t.send, &mut t.timeout)
                //     .await
                Ok(())
            }
            TransportEvent::Closed(clean) => self.handle_close(clean).await,
        }
    }

    /// handle a timeout
    async fn handle_timeout(&mut self) -> Result<()> {
        let Some(t) = &mut self.transport else {
            unreachable!("handle_timeout should never be called without a timeout")
        };

        match &mut t.timeout {
            Timeout::Ping(_) => {
                let ping = MessageEnvelope {
                    payload: MessagePayload::Ping {},
                };
                t.send.send(ping).await?;
                // NOTE: do i need to drain anything? probably not
                // self.conn.drain(&mut *t.send).await?;
                t.timeout = Timeout::for_close();
            }
            Timeout::Close(_) => {
                t.send.close().await?;
                // TODO: handle close, emit detach event
            }
        };
        Ok(())
    }

    async fn handle_close(&mut self, clean: bool) -> Result<()> {
        if clean {
            // set presence to offline
            // clear document presence
        }

        // self.conn.disconnect();
        // debug!("dehydrating syncer: {}", conn.get_id());
        // do something with s.services.connections

        // TODO: implement
        Ok(())
    }

    async fn handle_command(&mut self, command: Command) -> Result<()> {
        todo!()
    }
}

impl ConnectionHandle {
    pub fn id(&self) -> ConnectionId {
        self.id
    }

    pub fn session_id(&self) -> SessionId {
        todo!()
    }

    /// attach a transport to this connection
    pub fn attach(&self, transport: Box<dyn Transport>) {
        todo!()
    }

    /// shutdown this connection
    pub fn shutdown(&self) {
        todo!()
    }

    // /// stream events from this connection?
    // pub fn events(&self) { todo!() }
}

// TODO: later
// /// utility to accept new sync connections and do handshakes on them (wait for `Hello`)
// pub struct Handshake {
//     // ...
// }
//
// impl Handshake {
//     pub fn new(transport: AnyTransport) -> Self {
//         todo!()
//     }
//
//     pub async fn finish(self) -> Connection {
//         todo!()
//     }
// }
