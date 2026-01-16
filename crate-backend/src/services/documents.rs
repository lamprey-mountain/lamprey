use std::sync::Arc;

use base64::{prelude::BASE64_URL_SAFE_NO_PAD, Engine};
use common::v1::types::{ChannelId, DocumentBranchId, MessageSync, UserId};
use dashmap::DashMap;
use tokio::sync::RwLock;
use yrs::{
    updates::{decoder::Decode, encoder::Encode},
    Doc, GetString, ReadTxn, StateVector, Text, Transact, Update, Xml, XmlElementPrelim,
    XmlFragment,
};

use crate::{Result, ServerStateInner};

mod validate;

pub type EditContextId = (ChannelId, DocumentBranchId);

pub struct ServiceDocuments {
    state: Arc<ServerStateInner>,
    edit_contexts: DashMap<EditContextId, Arc<RwLock<EditContext>>>,
}

struct EditContext {
    /// the live crdt document
    doc: Doc,
    status: ContextStatus,
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
    pub async fn load(&self, context_id: EditContextId) -> Result<Arc<RwLock<EditContext>>> {
        let entry = match self.edit_contexts.entry(context_id) {
            dashmap::Entry::Occupied(o) => Arc::clone(o.get()),
            dashmap::Entry::Vacant(_v) => {
                // TODO: load doc from postgres

                // if the doc doesn't exist yet...
                let doc = Doc::new();
                doc.get_or_insert_xml_fragment("doc");
                let ctx = Arc::new(RwLock::new(EditContext {
                    doc,
                    status: ContextStatus::Open {},
                }));
                // TODO: save in postgres
                ctx
            }
        };
        Ok(entry)
    }

    /// apply a patch to a document
    pub async fn patch(&self, context_id: EditContextId, diff: &[u8]) -> Result<()> {
        let update = Update::decode_v1(diff).unwrap();
        let ctx = self.load(context_id).await?;
        let ctx = ctx.write().await;
        ctx.doc.transact_mut().apply_update(update).unwrap();
        drop(ctx);
        self.state
            .broadcast_channel(
                context_id.0,
                UserId::new(), // this is ignored, i should really remove user_id!
                MessageSync::DocumentEdit {
                    channel_id: context_id.0,
                    branch_id: context_id.1,
                    update: BASE64_URL_SAFE_NO_PAD.encode(&diff),
                },
            )
            .await?;
        Ok(())
    }

    pub async fn diff(&self, context_id: EditContextId, state_vector: &[u8]) -> Result<Vec<u8>> {
        let s = StateVector::decode_v1(state_vector).unwrap();
        let ctx = self.load(context_id).await?;
        let ctx = ctx.read().await;
        let serialized = ctx.doc.transact().encode_diff_v1(&s);
        Ok(serialized)
    }

    // pub async fn fork(&self) -> Result<()>;
    // pub async fn merge(&self) -> Result<()>;

    // /// remove dead edit contexts
    // pub async fn cleanup(&self) -> Result<()>;
}

struct DocumentSyncer {
    context_id: Option<EditContextId>,
}

//
// enum ActorMessage {
//     GetInitialRanges {
//         user_id: UserId,
//         ranges: Vec<(u64, u64)>,
//         callback: oneshot::Sender<MessageSync>,
//     },
// }

impl DocumentSyncer {
    /// set the edit context id for this syncer
    pub async fn set_context_id(&self, context_id: Option<EditContextId>) {
        // debug!("set user_id to {user_id:?}");
        // *self.user_id.lock().await = user_id;
    }

    pub async fn poll(&mut self) -> Result<MessageSync> {
        // MessageSync::DocumentEdit { channel_id: () };
        // MessageSync::DocumentPresence { channel_id: () };
        todo!()
    }
}

enum ContextStatus {
    /// at least one person is connected to this document
    Open {
        // last_snapshot_at: Time,
    },

    /// at least one person is connected to this document
    Closing {
        // closing_since: Time,
    },

    /// this document is dead and should be cleaned up
    Dead {
        // dead_since: Time
    },
}

impl ContextStatus {
    pub fn should_commit(&self) -> bool {
        // - if commit while Closing, set state to Dead?
        todo!()
    }

    // pub fn set_open(&mut self);
    // pub fn set_closing(&mut self);
    // pub fn set_dead(&mut self);
}

// struct Asdf {
//     /// the number of changes since the last snapshot
//     changes_since_last_snapshot: u64,

//     /// changes that have not been persisted yet
//     pending_changes: Vec<Change>,
// }

// trait Foo {
//     fn handle_change(change: &[u8]) {
//         // write change to a log; flush to postgres occasionally
//         // immediately update document
//     }
// }

// struct SnapshotLimiter {
//     // - every N updates (eg. 256)
//     // - every N seconds (eg. 30s)
//     // - when all clients disconnect (after some debounce time, eg. 5s)
// }

// impl SnapshotLimiter {
//     /// whether we should create a new snapshot
//     pub fn should_snapshot(&self) -> bool {
//         todo!()
//     }

//     pub fn snapshotted(&mut self) -> bool {
//         todo!()
//     }
// }
