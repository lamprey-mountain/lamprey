use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    time::{Duration, Instant},
};

use common::v1::types::document::serialized::Serdoc;
use kameo::{
    prelude::{Context, Message},
    Actor,
};
use lamprey_backend_core::{Error, Result};
use lamprey_backend_data_postgres::{ConnectionId, UserId};
use tokio::sync::broadcast;
use tracing::{debug, warn};
use uuid::Uuid;
use yrs::updates::encoder::Encode;
use yrs::{
    types::{Delta, Event},
    updates::decoder::Decode,
    DeepObservable, Doc, Out, ReadTxn, StateVector, Transact, Update,
};

use crate::{
    services::documents::{util::get_update_len, DocumentEvent, EditContextId, DOCUMENT_ROOT_NAME},
    ServerStateInner,
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
    pub cursor_head: String,
    pub cursor_tail: Option<String>,
}

/// a yjs/yrs crdt with presence
#[derive(Actor)]
pub struct DocumentActor {
    pub(super) context_id: EditContextId,
    pub(super) state: Arc<ServerStateInner>,

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

/// apply an edit to this edit context
pub struct ApplyUpdate {
    pub author_id: UserId,
    pub origin_conn_id: Option<ConnectionId>,
    pub update_bytes: Vec<u8>,
}

/// check if this document should be unloaded
pub struct CheckUnload;

/// get the current snapshot
pub struct GetSnapshot;

/// get the diff from a state vector
pub struct GetDiff {
    pub state_vector: StateVector,
}

/// broadcast presence update
pub struct BroadcastPresence {
    pub user_id: UserId,
    pub origin_conn_id: Option<ConnectionId>,
    pub cursor_head: String,
    pub cursor_tail: Option<String>,
}

/// remove presence
pub struct PresenceDelete {
    pub user_id: UserId,
    pub conn_id: ConnectionId,
}

/// get all presence
pub struct PresenceGet;

/// get the document content as a Serdoc
pub struct SerdocGet;

/// replace the document content from a Serdoc
pub struct SerdocPut {
    pub author_id: UserId,
    pub serdoc: Serdoc,
}

/// get document state vector
pub struct GetStateVector;

/// persist and unload this document
pub struct PersistAndUnload;

/// get a subscriber to the document event stream
pub struct Subscribe;

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
            let data = self.state.data();
            let new_seq = data
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
        self.last_flush = Instant::now();
        Ok(())
    }

    async fn snapshot(&mut self) -> Result<()> {
        let data = self.state.data();
        let snapshot = self
            .doc
            .transact()
            .encode_state_as_update_v1(&StateVector::default());
        let snapshot_id = Uuid::now_v7();
        let seq = self.last_seq;

        data.document_compact(self.context_id, snapshot_id, seq, snapshot)
            .await?;
        self.changes_since_last_snapshot = 0;
        self.last_snapshot = Instant::now();
        Ok(())
    }
}

impl Message<ApplyUpdate> for DocumentActor {
    type Reply = Result<()>;

    async fn handle(
        &mut self,
        msg: ApplyUpdate,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let update = Update::decode_v1(&msg.update_bytes)
            .map_err(|_| Error::Internal("Invalid update bytes".to_string()))?;

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
        let broadcast_bytes = msg.update_bytes.clone();
        self.pending_changes.push_back(PendingChange {
            author_id: msg.author_id,
            change: msg.update_bytes,
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
            origin_conn_id: msg.origin_conn_id,
            update: broadcast_bytes,
        });

        Ok(())
    }
}

impl Message<CheckUnload> for DocumentActor {
    type Reply = bool;

    async fn handle(
        &mut self,
        _msg: CheckUnload,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.presence.is_empty() && self.last_active.elapsed() > Duration::from_secs(60)
    }
}

impl Message<GetSnapshot> for DocumentActor {
    type Reply = Result<Vec<u8>>;

    async fn handle(
        &mut self,
        _msg: GetSnapshot,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        Ok(self
            .doc
            .transact()
            .encode_state_as_update_v1(&StateVector::default()))
    }
}

impl Message<GetDiff> for DocumentActor {
    type Reply = Result<Vec<u8>>;

    async fn handle(&mut self, msg: GetDiff, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        Ok(self.doc.transact().encode_diff_v1(&msg.state_vector))
    }
}

impl Message<BroadcastPresence> for DocumentActor {
    type Reply = Result<()>;

    async fn handle(
        &mut self,
        msg: BroadcastPresence,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.last_active = Instant::now();
        if let Some(conn_id) = msg.origin_conn_id {
            self.presence.insert(
                msg.user_id,
                PresenceData {
                    conn_id,
                    cursor_head: msg.cursor_head.clone(),
                    cursor_tail: msg.cursor_tail.clone(),
                },
            );
        }
        let _ = self.update_tx.send(DocumentEvent::Presence {
            user_id: msg.user_id,
            origin_conn_id: msg.origin_conn_id,
            cursor_head: msg.cursor_head,
            cursor_tail: msg.cursor_tail,
        });
        Ok(())
    }
}

impl Message<PresenceDelete> for DocumentActor {
    type Reply = Result<()>;

    async fn handle(
        &mut self,
        msg: PresenceDelete,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if let Some(presence) = self.presence.get(&msg.user_id) {
            if presence.conn_id == msg.conn_id {
                self.presence.remove(&msg.user_id);
                if self.presence.is_empty() {
                    self.last_active = Instant::now();
                }
                let _ = self.update_tx.send(DocumentEvent::Presence {
                    user_id: msg.user_id,
                    origin_conn_id: Some(msg.conn_id),
                    cursor_head: "".to_string(),
                    cursor_tail: None,
                });
            }
        }
        Ok(())
    }
}

impl Message<PresenceGet> for DocumentActor {
    type Reply = Result<Vec<(UserId, String, Option<String>, ConnectionId)>>;

    async fn handle(
        &mut self,
        _msg: PresenceGet,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
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
}

impl Message<SerdocGet> for DocumentActor {
    type Reply = Result<Serdoc>;

    async fn handle(
        &mut self,
        _msg: SerdocGet,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        Ok(crate::services::documents::serdoc::doc_to_serdoc(&self.doc))
    }
}

impl Message<SerdocPut> for DocumentActor {
    type Reply = Result<()>;

    async fn handle(
        &mut self,
        msg: SerdocPut,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        use crate::services::documents::serdoc;

        // calculate stats
        let old_serdoc = serdoc::doc_to_serdoc(&self.doc);
        let stat_removed = old_serdoc
            .root
            .blocks
            .iter()
            .map(|b| match b {
                common::v1::types::document::serialized::SerdocBlock::Markdown { content } => {
                    content.chars().count()
                }
            })
            .sum::<usize>() as u32;

        let stat_added = msg
            .serdoc
            .root
            .blocks
            .iter()
            .map(|b| match b {
                common::v1::types::document::serialized::SerdocBlock::Markdown { content } => {
                    content.chars().count()
                }
            })
            .sum::<usize>() as u32;

        let update_out = Arc::new(std::sync::Mutex::new(Vec::new()));
        let update_out_inner = update_out.clone();

        let _sub_update = self.doc.observe_update_v1(move |_, event| {
            let mut u = update_out_inner.lock().unwrap();
            *u = event.update.to_vec();
        });

        serdoc::serdoc_apply_to_doc(&self.doc, &msg.serdoc);

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
            author_id: msg.author_id,
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
}

impl Message<GetStateVector> for DocumentActor {
    type Reply = Result<Vec<u8>>;

    async fn handle(
        &mut self,
        _msg: GetStateVector,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        Ok(self.doc.transact().state_vector().encode_v1())
    }
}

impl Message<PersistAndUnload> for DocumentActor {
    type Reply = Result<()>;

    async fn handle(
        &mut self,
        _msg: PersistAndUnload,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let data = self.state.data();

        // flush changes
        while let Some(change) = self.pending_changes.pop_front() {
            let new_seq = data
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

            data.document_compact(self.context_id, snapshot_id, seq, snapshot)
                .await?;
        }

        Ok(())
    }
}

impl Message<Subscribe> for DocumentActor {
    type Reply = broadcast::Receiver<DocumentEvent>;

    async fn handle(
        &mut self,
        _msg: Subscribe,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.update_tx.subscribe()
    }
}
