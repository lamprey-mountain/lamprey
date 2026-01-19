use std::sync::Arc;

use base64::{prelude::BASE64_URL_SAFE_NO_PAD, Engine};
use common::v1::types::{ChannelId, DocumentBranchId, MessageSync, UserId};
use dashmap::DashMap;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;
use yrs::{updates::decoder::Decode, Doc, ReadTxn, StateVector, Transact, Update};

use crate::{Error, Result, ServerStateInner};

// mod validate;

pub type EditContextId = (ChannelId, DocumentBranchId);

pub struct ServiceDocuments {
    state: Arc<ServerStateInner>,
    edit_contexts: DashMap<EditContextId, Arc<RwLock<EditContext>>>,
}

#[derive(Clone, Debug)]
pub enum DocumentEvent {
    Update {
        origin_conn_id: Option<Uuid>,
        update: Vec<u8>,
    },
    Presence {
        user_id: UserId,
        origin_conn_id: Option<Uuid>,
        cursor_head: String,
        cursor_tail: Option<String>,
    },
}

pub struct EditContext {
    /// the live crdt document
    doc: Doc,

    /// the number of changes since the last snapshot
    changes_since_last_snapshot: u64,

    /// changes that have not been persisted yet
    pending_changes: Vec<PendingChange>,

    /// the sequence number of the last persisted update or snapshot
    last_seq: u32,

    update_tx: broadcast::Sender<DocumentEvent>,
}

struct PendingChange {
    author_id: UserId,
    change: Vec<u8>,
}

// TODO: better error handling (add yrs errors to to crate::Error)
impl ServiceDocuments {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            edit_contexts: DashMap::new(),
        }
    }

    /// load a document. reads from postgres if its not already in memory
    pub async fn load(
        &self,
        context_id: EditContextId,
        maybe_author: Option<UserId>,
    ) -> Result<Arc<RwLock<EditContext>>> {
        if let Some(ctx) = self.edit_contexts.get(&context_id) {
            return Ok(Arc::clone(&ctx));
        }

        let data = self.state.data();
        let loaded = data.document_load(context_id).await;

        let ctx = match loaded {
            Ok(dehydrated) => {
                // load an existing document
                let doc = Doc::new();
                doc.get_or_insert_xml_fragment("doc");
                let mut tx = doc.transact_mut();

                let snapshot = Update::decode_v1(&dehydrated.last_snapshot)
                    .map_err(|_| Error::Internal("failed to decode snapshot".to_owned()))?;
                tx.apply_update(snapshot).unwrap(); // TODO: better error handling

                for change in &dehydrated.changes {
                    let update = Update::decode_v1(change)
                        .map_err(|_| Error::Internal("failed to decode change".to_owned()))?;
                    tx.apply_update(update).unwrap();
                }
                drop(tx);

                let (update_tx, _) = broadcast::channel(100);

                Arc::new(RwLock::new(EditContext {
                    doc,
                    changes_since_last_snapshot: dehydrated.changes.len() as u64,
                    pending_changes: vec![],
                    last_seq: dehydrated.snapshot_seq,
                    update_tx,
                }))
            }
            Err(Error::NotFound) => {
                if let Some(author_id) = maybe_author {
                    let doc = Doc::new();
                    doc.get_or_insert_xml_fragment("doc");

                    let snapshot = doc
                        .transact()
                        .encode_state_as_update_v1(&StateVector::default());

                    data.document_create(context_id, author_id, snapshot)
                        .await?;

                    let (update_tx, _) = broadcast::channel(100);

                    Arc::new(RwLock::new(EditContext {
                        doc,
                        changes_since_last_snapshot: 0,
                        pending_changes: vec![],
                        last_seq: 0,
                        update_tx,
                    }))
                } else {
                    return Err(Error::NotFound);
                }
            }
            Err(e) => return Err(e),
        };

        match self.edit_contexts.entry(context_id) {
            dashmap::Entry::Occupied(o) => Ok(Arc::clone(o.get())),
            dashmap::Entry::Vacant(v) => {
                v.insert(Arc::clone(&ctx));
                Ok(ctx)
            }
        }
    }

    /// apply a change to a document
    pub async fn apply_update(
        &self,
        context_id: EditContextId,
        author_id: UserId,
        origin_conn_id: Option<Uuid>,
        update_bytes: &[u8],
    ) -> Result<()> {
        let update = Update::decode_v1(update_bytes).unwrap();
        let ctx = self.load(context_id, Some(author_id)).await?;
        let mut ctx = ctx.write().await;
        ctx.doc.transact_mut().apply_update(update).unwrap();
        ctx.changes_since_last_snapshot += 1;
        ctx.pending_changes.push(PendingChange {
            author_id,
            change: update_bytes.to_vec(),
        });

        let data = self.state.data();

        if ctx.should_flush() {
            let changes: Vec<_> = ctx.pending_changes.drain(..).collect();
            for change in changes {
                let new_seq = data
                    .document_update(context_id, change.author_id, change.change)
                    .await?;
                ctx.last_seq = new_seq;
            }
        }

        if ctx.should_snapshot() {
            let snapshot = ctx
                .doc
                .transact()
                .encode_state_as_update_v1(&StateVector::default());
            let snapshot_id = Uuid::now_v7();
            let seq = ctx.last_seq;

            data.document_compact(context_id, snapshot_id, seq, snapshot)
                .await?;
            ctx.changes_since_last_snapshot = 0;
        }

        let _ = ctx.update_tx.send(DocumentEvent::Update {
            origin_conn_id,
            update: update_bytes.to_vec(),
        });

        drop(ctx);
        Ok(())
    }

    pub async fn broadcast_presence(
        &self,
        context_id: EditContextId,
        user_id: UserId,
        origin_conn_id: Option<Uuid>,
        cursor_head: String,
        cursor_tail: Option<String>,
    ) -> Result<()> {
        if let Some(ctx) = self.edit_contexts.get(&context_id) {
            let ctx = ctx.read().await;
            let _ = ctx.update_tx.send(DocumentEvent::Presence {
                user_id,
                origin_conn_id,
                cursor_head,
                cursor_tail,
            });
        }
        Ok(())
    }

    pub async fn diff(&self, context_id: EditContextId, state_vector: &[u8]) -> Result<Vec<u8>> {
        let s = StateVector::decode_v1(state_vector).unwrap();
        let ctx = self.load(context_id, None).await?;
        let ctx = ctx.read().await;
        let serialized = ctx.doc.transact().encode_diff_v1(&s);
        Ok(serialized)
    }

    pub async fn subscribe(
        &self,
        context_id: EditContextId,
    ) -> Result<broadcast::Receiver<DocumentEvent>> {
        let ctx = self.load(context_id, None).await?;
        let ctx = ctx.read().await;
        Ok(ctx.update_tx.subscribe())
    }

    /// create a new DocumentSyncer for a session
    pub fn create_syncer(&self, conn_id: Uuid) -> DocumentSyncer {
        let (query_tx, query_rx) = tokio::sync::watch::channel(None);
        DocumentSyncer {
            s: self.state.clone(),
            query_tx,
            query_rx,
            current_rx: None,
            conn_id,
        }
    }
}

pub struct DocumentSyncer {
    s: Arc<ServerStateInner>,
    query_tx: tokio::sync::watch::Sender<Option<(EditContextId, Option<Vec<u8>>)>>,
    query_rx: tokio::sync::watch::Receiver<Option<(EditContextId, Option<Vec<u8>>)>>,
    current_rx: Option<(EditContextId, broadcast::Receiver<DocumentEvent>)>,
    conn_id: Uuid,
}

impl DocumentSyncer {
    /// set the edit context id for this syncer
    pub async fn set_context_id(
        &self,
        context_id: EditContextId,
        state_vector: Option<Vec<u8>>,
    ) -> Result<()> {
        self.query_tx
            .send(Some((context_id, state_vector)))
            .unwrap();
        Ok(())
    }

    pub async fn poll(&mut self) -> Result<MessageSync> {
        loop {
            if self.query_rx.has_changed().unwrap_or(false) {
                let _ = self.query_rx.borrow_and_update();
                let query = self.query_rx.borrow().clone();

                match query {
                    Some((context_id, state_vector)) => {
                        let rx = self.s.services().documents.subscribe(context_id).await?;
                        self.current_rx = Some((context_id, rx));

                        let srv = self.s.services();
                        let update = if let Some(sv) = state_vector {
                            srv.documents.diff(context_id, &sv).await?
                        } else {
                            let ctx = srv.documents.load(context_id, None).await?;
                            let ctx = ctx.read().await;
                            let update = ctx
                                .doc
                                .transact()
                                .encode_state_as_update_v1(&StateVector::default());
                            update
                        };

                        return Ok(MessageSync::DocumentEdit {
                            channel_id: context_id.0,
                            branch_id: context_id.1,
                            update: BASE64_URL_SAFE_NO_PAD.encode(&update),
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
                                    if origin_conn_id.as_ref() == Some(&self.conn_id) {
                                        continue;
                                    }
                                    return Ok(MessageSync::DocumentEdit {
                                        channel_id: context_id.0,
                                        branch_id: context_id.1,
                                        update: BASE64_URL_SAFE_NO_PAD.encode(&update),
                                    });
                                }
                                DocumentEvent::Presence {
                                    user_id,
                                    origin_conn_id,
                                    cursor_head,
                                    cursor_tail,
                                } => {
                                    if origin_conn_id.as_ref() == Some(&self.conn_id) {
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
                self.query_rx.changed().await.unwrap();
                continue;
            }
        }
    }
}

// TODO: fine tune these numbers
impl EditContext {
    /// whether we should create a new snapshot
    pub fn should_snapshot(&self) -> bool {
        // - every N updates (eg. 256)
        // - every N seconds (eg. 30s)
        // - when all clients disconnect (after some debounce time, eg. 5s)
        self.changes_since_last_snapshot > 100
    }

    /// whether we should flush pending_changes to db
    pub fn should_flush(&self) -> bool {
        // TODO: flush if time since last flush > 15s
        !self.pending_changes.is_empty()
    }
}
