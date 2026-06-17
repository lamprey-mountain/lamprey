// TODO: impl this

use std::collections::HashMap;

use common::{
    v1::types::{MessageSync, document::DocumentUpdate},
    v2::types::{ChannelId, DocumentBranchId, UserId},
};
use futures_util::{StreamExt, stream::BoxStream};
use yrs::Transact;

use crate::Client;

pub struct Documents {
    // TODO: for when DocumentSubscribe allows subscribing to multiple documents
    // manages document subscriptions
    // eg. sends MessageClient::DocumentSubscribe
    // unsubscribes on Document drop
}

/// a connection to a document
pub struct Document {
    // NOTE: maybe move some stuff into shared DocumentState, idk. or maybe its fine to have Document be owned
    // state: Arc<DocumentState>,
    channel_id: ChannelId,
    branch_id: DocumentBranchId,
    doc: yrs::Doc,
    presence: DocumentPresence,
}

pub struct DocumentPresence {
    // NOTE: maybe use dashmap, idk
    users: HashMap<UserId, DocumentCursor>,
}

// TODO: don't use String
pub struct DocumentCursor {
    head: String,
    tail: Option<String>,
}

pub enum DocumentEvent {
    Edit(DocumentUpdate),
    Presence(UserId, DocumentCursor),
    Subscribed,
    Disconnected,
}

impl Document {
    fn new() -> Self {
        // maybe expose doc directly? then maybe use observe to automatically send updates to the server?
        // let doc = yrs::Doc::new();
        // doc.observe
        // doc.observe_subdocs(f)
        // doc.transact();
        // doc.transact_mut();
        todo!()
    }

    pub fn edit(&self) {
        // DocumentUpdate;
        todo!()
    }

    pub fn update_presence(&self, _cursor: DocumentCursor) {
        todo!()
    }

    pub fn disconnect(&self) {
        todo!()
    }

    pub fn events(&self) -> BoxStream<'static, DocumentEvent> {
        todo!()
    }
}

// which http routes would i include here?
impl Document {
    // .routes(routes2!(wiki_history))
    // .routes(routes2!(document_branch_list))
    // .routes(routes2!(document_branch_get))
    // .routes(routes2!(document_branch_update))
    // .routes(routes2!(document_branch_sync))
    // .routes(routes2!(document_branch_close))
    // .routes(routes2!(document_branch_fork))
    // .routes(routes2!(document_branch_merge))
    // .routes(routes2!(document_tag_create))
    // .routes(routes2!(document_tag_list))
    // .routes(routes2!(document_tag_get))
    // .routes(routes2!(document_tag_update))
    // .routes(routes2!(document_tag_delete))
    // .routes(routes2!(document_history))
    // .routes(routes2!(document_crdt_diff))
    // .routes(routes2!(document_crdt_apply))
    // .routes(routes2!(document_content_get))
    // .routes(routes2!(document_content_put))
}

pub struct DocumentBuilder {
    channel_id: ChannelId,
}

impl DocumentBuilder {
    pub async fn connect(self) -> Document {
        todo!()
    }
}

impl Client {
    /// create a document connection
    pub fn document(&self, channel_id: ChannelId) -> DocumentBuilder {
        // self.syncer().sync().filter_map(|s| match s {
        //     MessageSync::DocumentSubscribed {
        //         channel_id,
        //         branch_id,
        //         connection_id,
        //     } => todo!(),
        //     MessageSync::DocumentEdit {
        //         channel_id,
        //         branch_id,
        //         update,
        //     } => todo!(),
        //     MessageSync::DocumentPresence {
        //         channel_id,
        //         branch_id,
        //         user_id,
        //         cursor_head,
        //         cursor_tail,
        //     } => todo!(),
        //     _ => todo!(),
        // });

        todo!()
    }
}
