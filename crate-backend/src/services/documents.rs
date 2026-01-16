use std::sync::Arc;

use base64::{prelude::BASE64_URL_SAFE_NO_PAD, Engine};
use common::v1::types::{ChannelId, DocumentBranchId, MessageSync, UserId};
use dashmap::DashMap;
use tokio::sync::RwLock;
use uuid::Uuid;
use yrs::{updates::decoder::Decode, Doc, ReadTxn, StateVector, Transact, Update};

use crate::{Error, Result, ServerStateInner};

mod validate;

pub type EditContextId = (ChannelId, DocumentBranchId);

pub struct ServiceDocuments {
    state: Arc<ServerStateInner>,
    edit_contexts: DashMap<EditContextId, Arc<RwLock<EditContext>>>,
}

struct EditContext {
    /// the live crdt document
    doc: Doc,

    #[allow(dead_code)] // TODO: use this
    status: EditContextStatus,

    /// the number of changes since the last snapshot
    changes_since_last_snapshot: u64,

    /// changes that have not been persisted yet
    pending_changes: Vec<PendingChange>,

    /// the sequence number of the last persisted update or snapshot
    last_seq: u32,
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

                Arc::new(RwLock::new(EditContext {
                    doc,
                    status: EditContextStatus::Open {},
                    changes_since_last_snapshot: dehydrated.changes.len() as u64,
                    pending_changes: vec![],
                    last_seq: dehydrated.snapshot_seq,
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

                    Arc::new(RwLock::new(EditContext {
                        doc,
                        status: EditContextStatus::Open {},
                        changes_since_last_snapshot: 0,
                        pending_changes: vec![],
                        last_seq: 0,
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

        drop(ctx);
        self.state
            .broadcast_channel(
                context_id.0,
                author_id,
                MessageSync::DocumentEdit {
                    channel_id: context_id.0,
                    branch_id: context_id.1,
                    update: BASE64_URL_SAFE_NO_PAD.encode(&update_bytes),
                },
            )
            .await?;
        Ok(())
    }

    pub async fn diff(&self, context_id: EditContextId, state_vector: &[u8]) -> Result<Vec<u8>> {
        let s = StateVector::decode_v1(state_vector).unwrap();
        let ctx = self.load(context_id, None).await?;
        let ctx = ctx.read().await;
        let serialized = ctx.doc.transact().encode_diff_v1(&s);
        Ok(serialized)
    }
}

struct DocumentSyncer {
    #[allow(dead_code)] // TODO: use this
    context_id: Option<EditContextId>,
}

// enum ActorMessage {
//     GetInitialRanges {
//         user_id: UserId,
//         ranges: Vec<(u64, u64)>,
//         callback: oneshot::Sender<MessageSync>,
//     },
// }

impl DocumentSyncer {
    /// set the edit context id for this syncer
    pub async fn set_context_id(&self, _context_id: Option<EditContextId>) {
        // debug!("set context_id to {context_id:?}");
        // *self.context_id.lock().await = context_id;
    }

    pub async fn poll(&mut self) -> Result<MessageSync> {
        // MessageSync::DocumentEdit { channel_id: () };
        // MessageSync::DocumentPresence { channel_id: () };
        todo!()
    }
}

enum EditContextStatus {
    /// at least one person is connected to this document
    Open {
        // last_snapshot_at: Time,
    },

    /// at least one person is connected to this document
    #[allow(dead_code)] // TODO: use this
    Closing {
        // closing_since: Time,
    },

    /// this document is dead and should be cleaned up
    #[allow(dead_code)] // TODO: use this
    Dead {
        // dead_since: Time
    },
}

impl EditContextStatus {
    #[allow(dead_code)] // TODO: use this
    pub fn should_commit(&self) -> bool {
        // - if commit while Closing, set state to Dead?
        todo!()
    }

    // pub fn set_open(&mut self);
    // pub fn set_closing(&mut self);
    // pub fn set_dead(&mut self);
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
