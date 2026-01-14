use std::sync::Arc;

use common::v1::types::{ChannelId, DocumentBranchId, MessageSync, UserId};
use dashmap::DashMap;
use tokio::sync::RwLock;
use yrs::{
    updates::{decoder::Decode, encoder::Encode},
    Doc, GetString, ReadTxn, StateVector, Text, Transact, Update,
};

use crate::{Result, ServerStateInner};

type EditContextId = (ChannelId, DocumentBranchId);

pub struct ServiceDocuments {
    #[allow(unused)] // TEMP
    state: Arc<ServerStateInner>,

    #[allow(unused)] // TEMP
    edit_contexts: DashMap<EditContextId, Arc<RwLock<EditContext>>>,
}

struct EditContext {
    /// the live crdt document
    doc: Doc,
    // last_snapshot_at
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
                todo!("load from postgres")
            }
        };
        Ok(entry)
    }

    /// apply a patch to a document
    pub async fn patch(&self, context_id: EditContextId, diff: Vec<u8>) -> Result<()> {
        let update = Update::decode_v1(&diff).unwrap();
        let ctx = self.load(context_id).await?;
        let ctx = ctx.write().await;
        ctx.doc.transact_mut().apply_update(update).unwrap();
        drop(ctx);
        // let mut doc = yrs::Doc::new();
        // let txt = doc.get_or_insert_text("content");
        // let mut tx = doc.transact_mut();
        // txt.insert(&mut tx, 0, "hello, world!");
        // assert_eq!(txt.get_string(&doc.transact()), "hello, world!");
        // let ts = doc.transact().state_vector().encode_v1();
        // let diff = doc
        //     .transact()
        //     .encode_diff_v1(&StateVector::decode_v1(&ts).unwrap());
        // doc.transact_mut()
        //     .apply_update(Update::decode_v1(&diff).unwrap())
        //     .unwrap();
        // TODO: broadcast diff to all peers
        self.state
            .broadcast_channel(
                context_id.0,
                UserId::new(), // this is ignored anyways... i should really remove it!
                MessageSync::DocumentEdit {
                    channel_id: context_id.0,
                },
            )
            .await?;
        Ok(())
    }

    pub async fn diff(&self, context_id: EditContextId, diff: Vec<u8>) -> Result<()> {
        todo!()
    }
}

struct DocumentSyncer {
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
