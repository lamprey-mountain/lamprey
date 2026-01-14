use std::sync::Arc;

use common::v1::types::{ChannelId, DocumentBranchId};
use dashmap::DashMap;
use tokio::sync::RwLock;

use crate::{Result, ServerStateInner};

type EditContextId = (ChannelId, DocumentBranchId);

pub struct ServiceDocuments {
    #[allow(unused)] // TEMP
    state: Arc<ServerStateInner>,

    #[allow(unused)] // TEMP
    edit_contexts: DashMap<EditContextId, Arc<RwLock<EditContext>>>,
}

struct EditContext {
    // /// the live crdt document
    // doc: YDoc,
}

impl ServiceDocuments {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            edit_contexts: DashMap::new(),
        }
    }

    /// load a document into memory
    #[allow(unused)] // TEMP
    pub async fn load(&self, channel_id: ChannelId, branch_id: DocumentBranchId) -> Result<()> {
        todo!()
    }
}
