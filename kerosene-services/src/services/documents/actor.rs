use std::{
    collections::{HashMap, VecDeque},
    time::{Duration, Instant},
};

use common::{
    v1::types::{
        components::{Component, ComponentCanonical, ComponentCreate, ComponentType, IdAllocator},
        document::serialized::Serdoc,
    },
    v2::types::{ConnectionId, UserId},
};
use kameo::{
    Actor,
    actor::ActorRef,
    prelude::{Context, Message},
};
use lamprey_backend_core::{Error, Result};
use tokio::sync::broadcast;
use tracing::{debug, warn};
use yrs::updates::encoder::Encode;
use yrs::{
    DeepObservable, Doc, Out, ReadTxn, StateVector, Transact, Update,
    types::{Delta, Event},
    updates::decoder::Decode,
};

use crate::prelude::*;
use crate::services::documents::{
    DOCUMENT_ROOT_NAME, DocumentEvent, EditContextId, util::get_update_len,
};

/// A pending change to be persisted
pub struct PendingChange {
    pub author_id: UserId,
    pub change: Vec<u8>,
    pub stat_added: u32,
    pub stat_removed: u32,
}

/// Presence data for a user
#[derive(Clone, Debug)]
pub struct PresenceData {
    pub conn_id: ConnectionId,
    // PERF: use bytes for this if possible
    pub cursor_head: String,
    pub cursor_tail: Option<String>,
}

/// a yjs/yrs crdt with presence
#[derive(Actor)]
pub struct DocumentActor {
    pub(super) context_id: EditContextId,
    pub(super) state: Globals,

    /// the live crdt document
    pub(super) doc: Doc,

    /// the number of changes since the last snapshot
    pub(super) changes_since_last_snapshot: u64,

    /// changes that have not been persisted yet
    pub(super) pending_changes: VecDeque<PendingChange>,

    /// the sequence number of the last persisted update or snapshot
    pub(super) last_seq: u32,
    pub(super) update_tx: broadcast::Sender<DocumentEvent>,
    pub(super) presence: HashMap<UserId, PresenceData>,
    pub(super) last_snapshot: Instant,
    pub(super) last_flush: Instant,
    pub(super) last_active: Instant,
}

impl DocumentActor {
    fn should_snapshot(&self) -> bool {
        if self.changes_since_last_snapshot > 256 {
            return true;
        } else if self.changes_since_last_snapshot == 0 {
            return false;
        }
        if self.last_snapshot.elapsed() > Duration::from_secs(30) {
            return true;
        }
        if self.presence.is_empty() && self.last_active.elapsed() > Duration::from_secs(15) {
            return true;
        }
        false
    }

    fn should_flush(&self) -> bool {
        if self.pending_changes.is_empty() {
            return false;
        }
        self.last_flush.elapsed() > Duration::from_secs(10)
    }

    async fn flush(&mut self) -> Result<()> {
        while let Some(change) = self.pending_changes.pop_front() {
            let mut txn = self.state.begin().await?;
            let new_seq = txn
                .document_update(
                    self.context_id,
                    change.author_id,
                    change.change,
                    change.stat_added,
                    change.stat_removed,
                )
                .await?;
            txn.commit().await?;
            self.last_seq = new_seq;
        }
        self.last_flush = Instant::now();
        Ok(())
    }

    async fn snapshot(&mut self) -> Result<()> {
        let mut txn = self.state.begin().await?;
        let snapshot = self
            .doc
            .transact()
            .encode_state_as_update_v1(&StateVector::default());
        let snapshot_id = Uuid::now_v7();
        let seq = self.last_seq;

        txn.document_compact(self.context_id, snapshot_id, seq, snapshot)
            .await?;
        txn.commit().await?;
        self.changes_since_last_snapshot = 0;
        self.last_snapshot = Instant::now();
        Ok(())
    }
}

#[kameo::messages]
impl DocumentActor {
    /// get a broadcast receiver to the document event stream
    #[message]
    pub fn subscribe(&self) -> broadcast::Receiver<DocumentEvent> {
        self.update_tx.subscribe()
    }

    /// get document state vector
    #[message]
    pub fn get_state_vector(&self) -> Vec<u8> {
        self.doc.transact().state_vector().encode_v1()
    }

    /// check if this document should be unloaded
    #[message]
    pub fn should_unload(&self) -> bool {
        self.presence.is_empty() && self.last_active.elapsed() > Duration::from_secs(60)
    }

    /// get the current snapshot
    #[message]
    pub fn get_snapshot(&self) -> Result<Vec<u8>> {
        Ok(self
            .doc
            .transact()
            .encode_state_as_update_v1(&StateVector::default()))
    }

    /// broadcast presence update
    #[message]
    pub fn broadcast_presence(
        &mut self,
        user_id: UserId,
        origin_conn_id: Option<ConnectionId>,
        cursor_head: String,
        cursor_tail: Option<String>,
    ) -> Result<()> {
        self.last_active = Instant::now();
        if let Some(conn_id) = origin_conn_id {
            self.presence.insert(
                user_id,
                PresenceData {
                    conn_id,
                    cursor_head: cursor_head.clone(),
                    cursor_tail: cursor_tail.clone(),
                },
            );
        }
        let _ = self.update_tx.send(DocumentEvent::Presence {
            user_id,
            origin_conn_id,
            cursor_head,
            cursor_tail,
        });
        Ok(())
    }

    /// remove presence
    #[message]
    pub fn presence_delete(&mut self, user_id: UserId, conn_id: ConnectionId) -> Result<()> {
        if let Some(presence) = self.presence.get(&user_id) {
            if presence.conn_id == conn_id {
                self.presence.remove(&user_id);
                if self.presence.is_empty() {
                    self.last_active = Instant::now();
                }
                let _ = self.update_tx.send(DocumentEvent::Presence {
                    user_id,
                    origin_conn_id: Some(conn_id),
                    cursor_head: "".to_string(),
                    cursor_tail: None,
                });
            }
        }
        Ok(())
    }

    /// get all presence
    #[message]
    pub fn presence_get(&self) -> Result<Vec<(UserId, String, Option<String>, ConnectionId)>> {
        Ok(self
            .presence
            .iter()
            .map(|(uid, data)| {
                (
                    *uid,
                    data.cursor_head.clone(),
                    data.cursor_tail.clone(),
                    data.conn_id,
                )
            })
            .collect())
    }

    /// get the diff from a state vector
    #[message]
    pub fn get_diff(&self, state_vector: StateVector) -> Result<Vec<u8>> {
        Ok(self.doc.transact().encode_diff_v1(&state_vector))
    }

    /// get the document content as a Serdoc
    #[message]
    pub fn serdoc_get(&self) -> Result<Serdoc> {
        Ok(crate::services::documents::serialized::doc_to_serdoc(
            &self.doc,
        ))
    }

    /// replace the document content from a Serdoc
    #[message]
    pub async fn serdoc_put(
        &mut self,
        author_id: UserId,
        components: Vec<ComponentCreate>,
    ) -> Result<()> {
        use crate::services::documents::serialized;

        // calculate stats
        let old_serdoc = serialized::doc_to_serdoc(&self.doc);
        let stat_removed = old_serdoc
            .components
            .iter()
            .map(|c| match &c.ty {
                ComponentType::Text { content } => content.chars().count(),
                _ => 0,
            })
            .sum::<usize>() as u32;

        let stat_added = components
            .iter()
            .map(|c| match &c.ty {
                ComponentType::Text { content } => content.chars().count(),
                _ => 0,
            })
            .sum::<usize>() as u32;

        let update_out = Arc::new(std::sync::Mutex::new(Vec::new()));
        let update_out_inner = update_out.clone();

        let _sub_update = self.doc.observe_update_v1(move |_, event| {
            let mut u = update_out_inner.lock().unwrap();
            *u = event.update.to_vec();
        });

        let mut allocator = IdAllocator::new();
        let canonical_components: Vec<ComponentCanonical> = components
            .into_iter()
            .map(|c| {
                let id = allocator.allocate(c.id);
                match c.ty {
                    ComponentType::Text { content } => Component {
                        id,
                        ty: ComponentType::Text { content },
                        allow: None,
                    },
                    _ => unimplemented!("only text components are supported for now"),
                }
            })
            .collect();

        serialized::serdoc_apply_to_doc(&self.doc, &canonical_components);

        drop(_sub_update);

        let update_bytes = {
            let u = update_out.lock().unwrap();
            u.clone()
        };

        if update_bytes.is_empty() {
            return Ok(());
        }

        self.last_active = Instant::now();
        self.changes_since_last_snapshot += 1;

        // Clone for broadcast, move original into pending_changes
        let broadcast_bytes = update_bytes.clone();
        self.pending_changes.push_back(PendingChange {
            author_id,
            change: update_bytes,
            stat_added,
            stat_removed,
        });

        if self.should_flush() {
            self.flush().await?;
        }

        if self.should_snapshot() {
            self.snapshot().await?;
        }

        let _ = self.update_tx.send(DocumentEvent::Update {
            origin_conn_id: None,
            update: broadcast_bytes,
        });

        Ok(())
    }

    /// persist and unload this document
    #[message]
    pub async fn persist_and_unload(&mut self) -> Result<()> {
        let mut txn = self.state.begin().await?;

        // flush changes
        while let Some(change) = self.pending_changes.pop_front() {
            let new_seq = txn
                .document_update(
                    self.context_id,
                    change.author_id,
                    change.change,
                    change.stat_added,
                    change.stat_removed,
                )
                .await?;
            self.last_seq = new_seq;
        }

        // snapshot if needed
        if self.changes_since_last_snapshot > 0 {
            let snapshot = self
                .doc
                .transact()
                .encode_state_as_update_v1(&StateVector::default());
            let snapshot_id = Uuid::now_v7();
            let seq = self.last_seq;

            txn.document_compact(self.context_id, snapshot_id, seq, snapshot)
                .await?;
        }
        txn.commit().await?;

        Ok(())
    }

    /// apply an edit to this edit context
    #[message]
    pub async fn apply_update(
        &mut self,
        author_id: UserId,
        origin_conn_id: Option<ConnectionId>,
        update_bytes: Vec<u8>,
    ) -> Result<()> {
        let update = Update::decode_v1(&update_bytes)
            .map_err(|_| Error::Internal("Invalid update bytes".to_string()))?;

        // PERF: surely theres a better way than with a mutex?
        // maybe atomics...?
        let stats = Arc::new(std::sync::Mutex::new((0, 0)));
        let stats_inner = stats.clone();

        let xml = self.doc.get_or_insert_xml_fragment(DOCUMENT_ROOT_NAME);
        let _sub = xml.observe_deep(move |txn, events| {
            let mut stats = stats_inner.lock().unwrap();
            for e in events.iter() {
                match e {
                    Event::Text(e) => {
                        for change in e.delta(txn) {
                            match change {
                                Delta::Inserted(t, _) => stats.0 += get_update_len(t, txn),
                                Delta::Deleted(len) => stats.1 += (*len) as usize,
                                Delta::Retain(_, _) => {}
                            }
                        }
                    }
                    Event::XmlText(e) => {
                        for change in e.delta(txn) {
                            match change {
                                Delta::Inserted(t, _) => stats.0 += get_update_len(t, txn),
                                Delta::Deleted(len) => stats.1 += (*len) as usize,
                                Delta::Retain(_, _) => {}
                            }
                        }
                    }
                    Event::XmlFragment(e) => {
                        for change in e.delta(txn) {
                            match change {
                                yrs::types::Change::Added(values) => {
                                    for v in values {
                                        stats.0 += get_update_len(v, txn);
                                    }
                                }
                                yrs::types::Change::Removed(len) => stats.1 += (*len) as usize,
                                yrs::types::Change::Retain(_) => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        });

        let mut txn = self.doc.transact_mut();
        txn.apply_update(update)?;

        if !txn
            .root_refs()
            .all(|(name, out)| name == DOCUMENT_ROOT_NAME && matches!(out, Out::YXmlFragment(_)))
        {
            warn!("got invalid root ref for document");
            // FIXME: rollback and return error here
        }

        drop(txn);
        drop(_sub);

        let (stat_inserted, stat_deleted) = {
            let s = stats.lock().unwrap();
            (s.0 as u32, s.1 as u32)
        };

        debug!(stat_inserted, stat_deleted, "edit stats");

        self.last_active = Instant::now();
        self.changes_since_last_snapshot += 1;

        // Clone for broadcast, move original into pending_changes
        let broadcast_bytes = update_bytes.clone();
        self.pending_changes.push_back(PendingChange {
            author_id,
            change: update_bytes,
            stat_added: stat_inserted,
            stat_removed: stat_deleted,
        });

        if self.should_flush() {
            self.flush().await?;
        }

        if self.should_snapshot() {
            self.snapshot().await?;
        }

        let _ = self.update_tx.send(DocumentEvent::Update {
            origin_conn_id,
            update: broadcast_bytes,
        });

        Ok(())
    }
}

/// a handle to a document actor
#[derive(Clone)]
pub struct DocumentHandle {
    actor_ref: ActorRef<DocumentActor>,
}

impl DocumentHandle {
    pub(super) fn new(actor_ref: ActorRef<DocumentActor>) -> Self {
        Self { actor_ref }
    }

    /// get a broadcast receiver to the document event stream
    pub async fn subscribe(&self) -> Result<broadcast::Receiver<DocumentEvent>> {
        self.actor_ref
            .ask(Subscribe {})
            .send()
            .await
            // TODO: better error
            .map_err(|e| Error::Internal(e.to_string()))
    }

    pub async fn presence_upsert(
        &self,
        user_id: UserId,
        origin_conn_id: Option<ConnectionId>,
        cursor_head: String,
        cursor_tail: Option<String>,
    ) -> Result<()> {
        self.actor_ref
            .ask(BroadcastPresence {
                user_id,
                origin_conn_id,
                cursor_head,
                cursor_tail,
            })
            .send()
            .await
            .map_err(|e| Error::Internal(e.to_string()))
    }

    // TODO: better return type
    pub async fn presence_list(
        &self,
    ) -> Result<Vec<(UserId, String, Option<String>, ConnectionId)>> {
        self.actor_ref
            .ask(PresenceGet {})
            .send()
            .await
            .map_err(|e| Error::Internal(e.to_string()))
    }

    pub async fn presence_delete(&self, user_id: UserId, conn_id: ConnectionId) -> Result<()> {
        self.actor_ref
            .ask(PresenceDelete { user_id, conn_id })
            .send()
            .await
            .map_err(|e| Error::Internal(e.to_string()))
    }

    /// check if this document should be unloaded
    pub async fn should_unload(&self) -> Result<bool> {
        self.actor_ref
            .ask(ShouldUnload {})
            .send()
            .await
            .map_err(|e| Error::Internal(e.to_string()))
    }

    /// persist and unload this document
    pub async fn persist_and_unload(&self) -> Result<()> {
        self.actor_ref
            .ask(PersistAndUnload {})
            .send()
            .await
            .map_err(|e| Error::Internal(e.to_string()))
    }

    /// get the diff from a state vector
    pub async fn get_diff(&self, state_vector: StateVector) -> Result<Vec<u8>> {
        self.actor_ref
            .ask(GetDiff { state_vector })
            .send()
            .await
            .map_err(|e| Error::Internal(e.to_string()))
    }

    /// get the current snapshot
    pub async fn get_snapshot(&self) -> Result<Vec<u8>> {
        self.actor_ref
            .ask(GetSnapshot {})
            .send()
            .await
            .map_err(|e| Error::Internal(e.to_string()))
    }

    /// get document state vector
    pub async fn get_state_vector(&self) -> Result<Vec<u8>> {
        self.actor_ref
            .ask(GetStateVector {})
            .send()
            .await
            .map_err(|e| Error::Internal(e.to_string()))
    }

    /// replace the document content from a Serdoc
    pub async fn serdoc_put(
        &self,
        author_id: UserId,
        components: Vec<ComponentCreate>,
    ) -> Result<()> {
        self.actor_ref
            .ask(SerdocPut {
                author_id,
                components,
            })
            .send()
            .await
            .map_err(|e| Error::Internal(e.to_string()))
    }

    /// get the document content as a Serdoc
    pub async fn serdoc_get(&self) -> Result<Serdoc> {
        self.actor_ref
            .ask(SerdocGet {})
            .send()
            .await
            .map_err(|e| Error::Internal(e.to_string()))
    }

    /// apply an edit to this edit context
    pub async fn apply_update(
        &self,
        author_id: UserId,
        origin_conn_id: Option<ConnectionId>,
        update_bytes: Vec<u8>,
    ) -> Result<()> {
        self.actor_ref
            .ask(ApplyUpdate {
                author_id,
                origin_conn_id,
                update_bytes,
            })
            .send()
            .await
            .map_err(|e| Error::Internal(e.to_string()))
    }
}
