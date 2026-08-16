use std::collections::VecDeque;

use common::v1::types::{
    ConnectionId, MessageSync, UserId,
    document::{DocumentStateVector, DocumentUpdate},
};
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;
use tracing::{error, warn};

use crate::{
    prelude::*,
    services::documents::{DocumentEvent, EditContextId, ServiceDocuments},
};

/// Handles document synchronization for a single client connection.
///
/// This struct manages the lifecycle of document subscriptions for a connection,
/// including subscribing/unsubscribing from documents, broadcasting updates,
/// and tracking presence information.
pub struct DocumentSyncer {
    /// Reference to the server state for accessing document services
    pub(super) s: Globals,

    /// Sends subscription requests to switch to a different document context.
    /// When a client subscribes to a new document, the desired context ID and
    /// optional state vector are sent through this channel.
    pub(super) query_tx: tokio::sync::watch::Sender<Option<(EditContextId, Option<Vec<u8>>)>>,

    /// Receives subscription requests from `query_tx`. The poll() loop monitors
    /// this receiver for changes. When a new query arrives, it sets up a
    /// subscription to the requested document and moves the subscription to `current_rx`.
    pub(super) query_rx: tokio::sync::watch::Receiver<Option<(EditContextId, Option<Vec<u8>>)>>,

    /// The active document subscription. Contains the current document context ID
    /// and a broadcast receiver for receiving document events (updates and presence).
    /// When switching documents, the old subscription is replaced with a new one.
    pub(super) current_rx: Option<(EditContextId, broadcast::Receiver<DocumentEvent>)>,

    /// The connection ID associated with this syncer, used to filter out
    /// self-originated events and track presence.
    pub(super) conn_id: ConnectionId,

    /// Queue of pending sync messages to be sent to the client. Used for buffering
    /// messages like initial presence data when first subscribing to a document.
    pub(super) pending_sync: VecDeque<MessageSync>,

    /// The user ID of the authenticated user. Required for document operations
    /// and presence tracking.
    pub(super) user_id: Option<UserId>,
}

impl DocumentSyncer {
    pub async fn set_user_id(&mut self, user_id: Option<UserId>) {
        self.user_id = user_id;
    }

    /// set edit context id for this syncer
    pub async fn set_context_id(
        &self,
        context_id: EditContextId,
        state_vector: Option<DocumentStateVector>,
    ) -> Result<()> {
        self.query_tx
            .send(Some((context_id, state_vector.map(|sv| sv.0))))
            .map_err(|_| Error::Internal("query channel closed".to_string()))?;
        Ok(())
    }

    /// Check if client is actively subscribed to a document.
    ///
    /// This checks `current_rx` (the active subscription) rather than `query_rx`
    /// (the pending subscription request). This distinction matters when switching
    /// documents: a client is only considered "subscribed" after the subscription
    /// has been fully established and is being polled.
    pub fn is_subscribed(&self, context_id: &EditContextId) -> bool {
        self.current_rx
            .as_ref()
            .map(|(current_id, _)| current_id == context_id)
            .unwrap_or(false)
    }

    pub async fn handle_disconnect(&self, user_id: UserId) -> Result<()> {
        if let Some((context_id, _)) = &self.current_rx {
            self.s
                .services()
                .documents
                .remove_presence(*context_id, user_id, self.conn_id)
                .await?;
        }
        Ok(())
    }

    pub async fn poll(&mut self) -> Result<MessageSync> {
        loop {
            if let Some(msg) = self.pending_sync.pop_front() {
                return Ok(msg);
            }

            if self.query_rx.has_changed().unwrap_or(false) {
                let _ = self.query_rx.borrow_and_update();
                let query = self.query_rx.borrow().clone();

                match query {
                    Some((context_id, state_vector)) => {
                        // TODO: check that self.user_id is Some

                        let rx = self
                            .s
                            .services()
                            .documents
                            .subscribe(context_id, self.user_id)
                            .await?;
                        self.current_rx = Some((context_id, rx));

                        let srv = self.s.services();
                        let update = if let Some(sv) = state_vector {
                            srv.documents.diff(context_id, self.user_id, &sv).await?
                        } else {
                            srv.documents.get_snapshot(context_id).await?
                        };

                        let presences = srv.documents.get_presence(context_id).await?;
                        for (user_id, cursor_head, cursor_tail, conn_id) in presences {
                            if conn_id != self.conn_id {
                                self.pending_sync.push_back(MessageSync::DocumentPresence {
                                    channel_id: context_id.channel_id(),
                                    branch_id: context_id.branch_id(),
                                    user_id,
                                    cursor_head,
                                    cursor_tail,
                                });
                            }
                        }

                        // Queue DocumentSubscribed to be sent after the initial DocumentEdit
                        self.pending_sync
                            .push_back(MessageSync::DocumentSubscribed {
                                channel_id: context_id.channel_id(),
                                branch_id: context_id.branch_id(),
                                connection_id: self.conn_id,
                            });

                        return Ok(MessageSync::DocumentEdit {
                            channel_id: context_id.channel_id(),
                            branch_id: context_id.branch_id(),
                            update: DocumentUpdate(update),
                        });
                    }
                    None => {
                        self.current_rx = None;
                        continue;
                    }
                }
            }

            if let Some((context_id, rx)) = &mut self.current_rx {
                tokio::select! {
                    res = rx.recv() => {
                        match res {
                            Ok(event) => match event {
                                DocumentEvent::Update { origin_conn_id, update } => {
                                    if origin_conn_id == Some(self.conn_id) {
                                        continue;
                                    }
                                    return Ok(MessageSync::DocumentEdit {
                                        channel_id: context_id.channel_id(),
                                        branch_id: context_id.branch_id(),
                                        update: DocumentUpdate(update),
                                    });
                                }
                                DocumentEvent::Presence {
                                    user_id,
                                    origin_conn_id,
                                    cursor_head,
                                    cursor_tail,
                                } => {
                                    if origin_conn_id == Some(self.conn_id) {
                                        continue;
                                    }
                                    return Ok(MessageSync::DocumentPresence {
                                        channel_id: context_id.channel_id(),
                                        branch_id: context_id.branch_id(),
                                        user_id,
                                        cursor_head,
                                        cursor_tail,
                                    });
                                }
                            },
                            Err(RecvError::Closed) => {
                                error!("sender died, unsubscribind");
                                self.current_rx = None;
                                continue;
                            }
                            Err(RecvError::Lagged(n)) => {
                                warn!("receiver lagged and skipped {n} messages");
                                continue;
                            }
                        }
                    }
                    _ = self.query_rx.changed() => continue,
                }
            } else {
                self.query_rx
                    .changed()
                    .await
                    .map_err(|_| Error::Internal("query channel closed".to_string()))?;
                continue;
            }
        }
    }
}

impl ServiceDocuments {
    /// create a new DocumentSyncer for a session
    pub fn create_syncer(&self, conn_id: ConnectionId) -> DocumentSyncer {
        let (query_tx, query_rx) = tokio::sync::watch::channel(None);
        DocumentSyncer {
            s: self.globals.clone(),
            query_tx,
            query_rx,
            current_rx: None,
            conn_id,
            pending_sync: VecDeque::new(),
            user_id: None,
        }
    }
}
