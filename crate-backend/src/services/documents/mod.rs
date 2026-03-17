use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use common::v1::types::document::serialized::Serdoc;
use common::v1::types::document::{Changeset, DocumentTag, HistoryParams};
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::{
    document::{DocumentStateVector, DocumentUpdate},
    ChannelId, ConnectionId, DocumentBranchId, MessageSync, UserId,
};
use dashmap::DashMap;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use kameo::actor::{ActorRef, Spawn};
use tokio::sync::broadcast;
use tracing::{debug, error};
use yrs::ReadTxn;
use yrs::{updates::decoder::Decode, Doc, StateVector, Transact, Update};

use crate::services::documents::actor::{
    ApplyUpdate, BroadcastPresence, CheckUnload, DocumentActor, GetDiff, GetSnapshot,
    GetStateVector, PersistAndUnload, PresenceDelete, PresenceGet, SerdocGet, SerdocPut, Subscribe,
};
use crate::services::documents::util::{HistoryPaginationSummary, DOCUMENT_ROOT_NAME};
use crate::types::DocumentUpdateSummary;
use crate::{Error, Result, ServerStateInner};

// mod validate;
mod actor;
mod serdoc;
mod util;

pub type EditContextId = (ChannelId, DocumentBranchId);

pub struct ServiceDocuments {
    state: Arc<ServerStateInner>,
    edit_contexts: DashMap<EditContextId, ActorRef<DocumentActor>>,
}

#[derive(Clone, Debug)]
pub enum DocumentEvent {
    Update {
        origin_conn_id: Option<ConnectionId>,
        update: Vec<u8>,
    },
    Presence {
        user_id: UserId,
        origin_conn_id: Option<ConnectionId>,
        cursor_head: String,
        cursor_tail: Option<String>,
    },
}

pub use actor::PendingChange;

// TODO: better error handling (add yrs errors to to crate::Error)
impl ServiceDocuments {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            edit_contexts: DashMap::new(),
        }
    }

    pub fn start_background_tasks(&self) {
        let state = self.state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                let services = state.services();
                let documents = &services.documents;

                // capture actors to avoid holding dashmap locks across await points
                let actors: Vec<_> = documents
                    .edit_contexts
                    .iter()
                    .map(|entry| (*entry.key(), entry.value().clone()))
                    .collect();

                let mut to_unload = Vec::new();
                for (id, actor_ref) in actors {
                    if let Ok(true) = actor_ref.ask(CheckUnload).send().await {
                        to_unload.push(id);
                    }
                }

                for id in to_unload {
                    if let Err(e) = documents.unload(id).await {
                        error!("failed to unload document {:?}: {}", id, e);
                    }
                }
            }
        });
    }

    /// load a document. reads from postgres if its not already in memory
    pub async fn load(
        &self,
        context_id: EditContextId,
        maybe_author: Option<UserId>,
    ) -> Result<ActorRef<DocumentActor>> {
        if let Some(actor_ref) = self.edit_contexts.get(&context_id) {
            return Ok(actor_ref.clone());
        }

        debug!(context_id = ?context_id, maybe_author = ?maybe_author, "load document");
        let data = self.state.data();
        let loaded = data.document_load(context_id).await;

        let actor_ref = match loaded {
            Ok(dehydrated) => {
                // load an existing document
                let doc = Doc::new();
                doc.get_or_insert_xml_fragment(DOCUMENT_ROOT_NAME);
                let mut tx = doc.transact_mut();

                let snapshot = Update::decode_v1(&dehydrated.last_snapshot)?;
                tx.apply_update(snapshot)?;

                for change in &dehydrated.changes {
                    let update = Update::decode_v1(change)?;
                    tx.apply_update(update)?;
                }
                drop(tx);

                let (update_tx, _) = broadcast::channel(100);

                let actor = DocumentActor {
                    context_id,
                    state: self.state.clone(),
                    doc,
                    changes_since_last_snapshot: dehydrated.changes.len() as u64,
                    pending_changes: VecDeque::new(),
                    last_seq: dehydrated.snapshot_seq,
                    update_tx,
                    presence: HashMap::new(),
                    last_snapshot: Instant::now(),
                    last_flush: Instant::now(),
                    last_active: Instant::now(),
                };
                DocumentActor::spawn(actor)
            }
            Err(Error::ApiError(ApiError {
                code: ErrorCode::UnknownDocumentBranch,
                ..
            })) => {
                if let Some(author_id) = maybe_author {
                    let doc = Doc::new();
                    doc.get_or_insert_xml_fragment(DOCUMENT_ROOT_NAME);

                    let snapshot = doc
                        .transact()
                        .encode_state_as_update_v1(&StateVector::default());

                    data.document_create(context_id, author_id, snapshot)
                        .await?;

                    let (update_tx, _) = broadcast::channel(100);

                    let actor = DocumentActor {
                        context_id,
                        state: self.state.clone(),
                        doc,
                        changes_since_last_snapshot: 0,
                        pending_changes: VecDeque::new(),
                        last_seq: 0,
                        update_tx,
                        presence: HashMap::new(),
                        last_snapshot: Instant::now(),
                        last_flush: Instant::now(),
                        last_active: Instant::now(),
                    };
                    DocumentActor::spawn(actor)
                } else {
                    return Err(Error::ApiError(ApiError::from_code(
                        ErrorCode::UnknownDocumentBranch,
                    )));
                }
            }
            Err(e) => return Err(e),
        };

        match self.edit_contexts.entry(context_id) {
            dashmap::Entry::Occupied(o) => Ok(o.get().clone()),
            dashmap::Entry::Vacant(v) => {
                v.insert(actor_ref.clone());
                Ok(actor_ref)
            }
        }
    }

    /// unload a document from memory
    // TODO: automatically unload unused documents
    pub async fn unload(&self, context_id: EditContextId) -> Result<()> {
        if let Some((_, actor_ref)) = self.edit_contexts.remove(&context_id) {
            actor_ref
                .ask(PersistAndUnload)
                .send()
                .await
                .map_err(|e| Error::Internal(format!("actor send error: {}", e)))?;
        }

        Ok(())
    }

    /// unload all documents, for shutting down
    pub async fn unload_all(&self) {
        let mut futures = FuturesUnordered::new();

        for entry in &self.edit_contexts {
            let context_id = *entry.key();
            futures.push(self.unload(context_id));
        }

        while let Some(r) = futures.next().await {
            if let Err(err) = r {
                error!("failed to unload document: {err:?}");
            }
        }
    }

    /// apply a change to a document
    #[tracing::instrument(skip(self, update_bytes))]
    pub async fn apply_update(
        &self,
        context_id: EditContextId,
        author_id: UserId,
        origin_conn_id: Option<ConnectionId>,
        update_bytes: &[u8],
    ) -> Result<()> {
        let actor_ref = self.load(context_id, Some(author_id)).await?;
        actor_ref
            .ask(ApplyUpdate {
                author_id,
                origin_conn_id,
                update_bytes: update_bytes.to_vec(),
            })
            .send()
            .await
            .map_err(|e| Error::Internal(format!("actor send error: {}", e)))
    }

    pub async fn get_content(&self, context_id: EditContextId) -> Result<Serdoc> {
        let actor_ref = self.load(context_id, None).await?;
        Ok(actor_ref
            .ask(SerdocGet)
            .send()
            .await
            .map_err(|e| Error::Internal(format!("actor send error: {}", e)))?)
    }

    pub async fn get_content_at_seq(&self, context_id: EditContextId, seq: u64) -> Result<Serdoc> {
        let data = self.state.data();
        let dehydrated = data.document_load_at_seq(context_id, seq as u32).await?;

        let doc = yrs::Doc::new();
        doc.get_or_insert_xml_fragment(DOCUMENT_ROOT_NAME);

        let mut txn = doc.transact_mut();

        // start with the last snapshot
        let snapshot = yrs::Update::decode_v1(&dehydrated.last_snapshot)?;
        txn.apply_update(snapshot)?;

        // replay updates
        for update_data in dehydrated.changes {
            let update = yrs::Update::decode_v1(&update_data)?;
            txn.apply_update(update)?;
        }
        drop(txn);

        Ok(serdoc::doc_to_serdoc(&doc))
    }

    pub async fn set_content(
        &self,
        context_id: EditContextId,
        author_id: UserId,
        content: Serdoc,
    ) -> Result<()> {
        let actor_ref = self.load(context_id, Some(author_id)).await?;
        actor_ref
            .ask(SerdocPut {
                author_id,
                serdoc: content,
            })
            .send()
            .await
            .map_err(|e| Error::Internal(format!("actor send error: {}", e)))
    }

    pub async fn broadcast_presence(
        &self,
        context_id: EditContextId,
        user_id: UserId,
        origin_conn_id: Option<ConnectionId>,
        cursor_head: String,
        cursor_tail: Option<String>,
    ) -> Result<()> {
        if let Some(actor_ref) = self.edit_contexts.get(&context_id) {
            actor_ref
                .ask(BroadcastPresence {
                    user_id,
                    origin_conn_id,
                    cursor_head,
                    cursor_tail,
                })
                .send()
                .await
                .map_err(|e| Error::Internal(format!("actor send error: {}", e)))?;
        }

        Ok(())
    }

    pub async fn remove_presence(
        &self,
        context_id: EditContextId,
        user_id: UserId,
        conn_id: ConnectionId,
    ) -> Result<()> {
        if let Some(actor_ref) = self.edit_contexts.get(&context_id) {
            actor_ref
                .ask(PresenceDelete { user_id, conn_id })
                .send()
                .await
                .map_err(|e| Error::Internal(format!("actor send error: {}", e)))?;
        }
        Ok(())
    }

    pub async fn get_presence(
        &self,
        context_id: EditContextId,
    ) -> Result<Vec<(UserId, String, Option<String>, ConnectionId)>> {
        let actor_ref = self.load(context_id, None).await?;
        Ok(actor_ref
            .ask(PresenceGet)
            .send()
            .await
            .map_err(|e| Error::Internal(format!("actor send error: {}", e)))?)
    }

    pub async fn diff(
        &self,
        context_id: EditContextId,
        maybe_author: Option<UserId>,
        state_vector: &[u8],
    ) -> Result<Vec<u8>> {
        let s = if state_vector.is_empty() {
            StateVector::default()
        } else {
            StateVector::decode_v1(state_vector)?
        };
        let actor_ref = self.load(context_id, maybe_author).await?;
        Ok(actor_ref
            .ask(GetDiff { state_vector: s })
            .send()
            .await
            .map_err(|e| Error::Internal(format!("actor send error: {}", e)))?)
    }

    pub async fn get_snapshot(&self, context_id: EditContextId) -> Result<Vec<u8>> {
        let actor_ref = self.load(context_id, None).await?;
        Ok(actor_ref
            .ask(GetSnapshot)
            .send()
            .await
            .map_err(|e| Error::Internal(format!("actor send error: {}", e)))?)
    }

    pub async fn get_state_vector(&self, context_id: EditContextId) -> Result<Vec<u8>> {
        let actor_ref = self.load(context_id, None).await?;
        Ok(actor_ref
            .ask(GetStateVector)
            .send()
            .await
            .map_err(|e| Error::Internal(format!("actor send error: {}", e)))?)
    }

    pub async fn subscribe(
        &self,
        context_id: EditContextId,
        maybe_author: Option<UserId>,
    ) -> Result<broadcast::Receiver<DocumentEvent>> {
        let actor_ref = self.load(context_id, maybe_author).await?;
        Ok(actor_ref
            .ask(Subscribe)
            .send()
            .await
            .map_err(|e| Error::Internal(format!("actor send error: {}", e)))?)
    }

    /// create a new DocumentSyncer for a session
    pub fn create_syncer(&self, conn_id: ConnectionId) -> DocumentSyncer {
        let (query_tx, query_rx) = tokio::sync::watch::channel(None);
        DocumentSyncer {
            s: self.state.clone(),
            query_tx,
            query_rx,
            current_rx: None,
            conn_id,
            pending_sync: VecDeque::new(),
            user_id: None,
        }
    }

    pub async fn query_history(
        &self,
        context_id: EditContextId,
        query: HistoryParams,
    ) -> Result<HistoryPaginationSummary> {
        let data = self.state.data();
        let (updates, tags) = data.document_history(context_id).await?;
        self.process_history(updates, tags, query)
    }

    pub async fn query_wiki_history(
        &self,
        wiki_id: ChannelId,
        query: HistoryParams,
    ) -> Result<HistoryPaginationSummary> {
        let data = self.state.data();
        let (updates, tags) = data.wiki_history(wiki_id).await?;
        self.process_history(updates, tags, query)
    }

    fn process_history(
        &self,
        updates: Vec<DocumentUpdateSummary>,
        tags: Vec<DocumentTag>,
        query: HistoryParams,
    ) -> Result<HistoryPaginationSummary> {
        let by_author = query.by_author.unwrap_or(true);
        let by_tag = query.by_tag.unwrap_or(true);
        let by_time = query.by_time.unwrap_or(3600) as i64;
        let by_changes = query.by_changes.unwrap_or(100) as usize;

        let mut changesets = Vec::new();
        if updates.is_empty() {
            return Ok(HistoryPaginationSummary {
                changesets,
                tags: vec![],
            });
        }

        let mut current_authors = HashSet::new();
        let mut current_added = 0;
        let mut current_removed = 0;
        let mut current_start = updates[0].created_at;
        let mut current_end = updates[0].created_at;
        let mut current_count = 0;
        let mut current_document_id = updates[0].document_id;
        let mut current_start_seq = updates[0].seq;
        let mut current_end_seq = updates[0].seq;

        let mut tag_iter = tags.iter().peekable();

        for (i, update) in updates.iter().enumerate() {
            let mut split = false;

            if i > 0 {
                let prev = &updates[i - 1];

                if update.document_id != prev.document_id {
                    split = true;
                }

                if by_author && update.user_id != prev.user_id {
                    split = true;
                }

                let diff = (*update.created_at - *prev.created_at).whole_seconds();
                if diff > by_time {
                    split = true;
                }

                if current_count >= by_changes {
                    split = true;
                }

                if by_tag {
                    while let Some(tag) = tag_iter.peek() {
                        if tag.revision_seq < prev.seq as u64 {
                            tag_iter.next();
                            continue;
                        }
                        if tag.revision_seq == prev.seq as u64 {
                            split = true;
                        }
                        break;
                    }
                }
            }

            if split {
                changesets.push(Changeset {
                    start_time: current_start,
                    end_time: current_end,
                    authors: current_authors.drain().collect(),
                    stat_added: current_added,
                    stat_removed: current_removed,
                    document_id: Some(current_document_id),
                    start_seq: current_start_seq,
                    end_seq: current_end_seq,
                });
                current_added = 0;
                current_removed = 0;
                current_count = 0;
                current_start = update.created_at;
                current_start_seq = update.seq;
                current_document_id = update.document_id;
            }

            current_authors.insert(UserId::from(update.user_id));
            current_added += update.stat_added as u64;
            current_removed += update.stat_removed as u64;
            current_end = update.created_at;
            current_end_seq = update.seq;
            current_count += 1;
        }

        changesets.push(Changeset {
            start_time: current_start,
            end_time: current_end,
            authors: current_authors.drain().collect(),
            stat_added: current_added,
            stat_removed: current_removed,
            document_id: Some(current_document_id),
            start_seq: current_start_seq,
            end_seq: current_end_seq,
        });

        changesets.reverse();

        if let Some(limit) = query.limit {
            changesets.truncate(limit as usize);
        } else {
            changesets.truncate(20);
        }

        Ok(HistoryPaginationSummary { changesets, tags })
    }
}

/// Handles document synchronization for a single client connection.
///
/// This struct manages the lifecycle of document subscriptions for a connection,
/// including subscribing/unsubscribing from documents, broadcasting updates,
/// and tracking presence information.
pub struct DocumentSyncer {
    /// Reference to the server state for accessing document services
    s: Arc<ServerStateInner>,

    /// Sends subscription requests to switch to a different document context.
    /// When a client subscribes to a new document, the desired context ID and
    /// optional state vector are sent through this channel.
    query_tx: tokio::sync::watch::Sender<Option<(EditContextId, Option<Vec<u8>>)>>,

    /// Receives subscription requests from `query_tx`. The poll() loop monitors
    /// this receiver for changes. When a new query arrives, it sets up a
    /// subscription to the requested document and moves the subscription to `current_rx`.
    query_rx: tokio::sync::watch::Receiver<Option<(EditContextId, Option<Vec<u8>>)>>,

    /// The active document subscription. Contains the current document context ID
    /// and a broadcast receiver for receiving document events (updates and presence).
    /// When switching documents, the old subscription is replaced with a new one.
    current_rx: Option<(EditContextId, broadcast::Receiver<DocumentEvent>)>,

    /// The connection ID associated with this syncer, used to filter out
    /// self-originated events and track presence.
    conn_id: ConnectionId,

    /// Queue of pending sync messages to be sent to the client. Used for buffering
    /// messages like initial presence data when first subscribing to a document.
    pending_sync: VecDeque<MessageSync>,

    /// The user ID of the authenticated user. Required for document operations
    /// and presence tracking.
    user_id: Option<UserId>,
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
                                    channel_id: context_id.0,
                                    branch_id: context_id.1,
                                    user_id,
                                    cursor_head,
                                    cursor_tail,
                                });
                            }
                        }

                        // Queue DocumentSubscribed to be sent after the initial DocumentEdit
                        self.pending_sync
                            .push_back(MessageSync::DocumentSubscribed {
                                channel_id: context_id.0,
                                branch_id: context_id.1,
                                connection_id: self.conn_id,
                            });

                        return Ok(MessageSync::DocumentEdit {
                            channel_id: context_id.0,
                            branch_id: context_id.1,
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
                                        channel_id: context_id.0,
                                        branch_id: context_id.1,
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
                                        channel_id: context_id.0,
                                        branch_id: context_id.1,
                                        user_id,
                                        cursor_head,
                                        cursor_tail,
                                    });
                                }
                            },
                            Err(_) => continue,
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
