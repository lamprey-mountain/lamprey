use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use kameo::{
    prelude::{Context, Message},
    Actor,
};
use lamprey_backend_core::{Error, Result};
use lamprey_backend_data_postgres::{ConnectionId, UserId};
use tokio::sync::broadcast;
use tracing::{debug, warn};
use uuid::Uuid;
use yrs::{
    types::{Delta, Event},
    updates::decoder::Decode,
    DeepObservable, Doc, Out, ReadTxn, StateVector, Transact, Update,
};

use crate::{
    services::documents::{
        util::{get_update_len, DOCUMENT_ROOT_NAME},
        DocumentEvent, EditContextId, PendingChange, PresenceData,
    },
    ServerStateInner,
};

/// a yjs/yrs crdt with presence
// rename to EditContext?
#[derive(Actor)]
pub struct DocumentActor {
    context_id: EditContextId,
    state: Arc<ServerStateInner>,
    /// the live crdt document
    doc: Doc,
    /// the number of changes since the last snapshot
    changes_since_last_snapshot: u64,
    /// changes that have not been persisted yet
    pending_changes: Vec<PendingChange>,
    /// the sequence number of the last persisted update or snapshot
    last_seq: u32,
    update_tx: broadcast::Sender<DocumentEvent>,
    presence: HashMap<UserId, PresenceData>,
    last_snapshot: Instant,
    last_flush: Instant,
    last_active: Instant,
}

/// apply an edit to this edit context
pub struct ApplyUpdate {
    pub author_id: UserId,
    pub origin_conn_id: Option<ConnectionId>,
    pub update_bytes: Vec<u8>,
}

/// check if this document should be unloaded
pub struct CheckUnload;

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
        if self.pending_changes.is_empty() {
            return Ok(());
        }
        let data = self.state.data();
        let changes: Vec<_> = self.pending_changes.drain(..).collect();
        for change in changes {
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
        ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
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
        self.pending_changes.push(PendingChange {
            author_id: msg.author_id,
            change: msg.update_bytes.clone(),
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
            update: msg.update_bytes,
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
