//! a connection to a document

use lamprey_macros::record;

use crate::{
    v1::types::document::{DocumentStateVector, DocumentUpdate},
    v2::types::{DocumentBranchId, DocumentId, UserId},
};

#[record]
pub struct Initial {
    /// the id of the document to subscribe to
    pub document_id: DocumentId,

    /// which branch of the document to subscribe to
    pub branch_id: Option<DocumentBranchId>,

    /// initial state vector
    ///
    /// if unset, the server will send the full state of the document
    pub state_vector: Option<DocumentStateVector>,
}

#[record]
pub enum Command {
    /// edit a document
    Edit { update: DocumentUpdate },

    /// update your presence in this document
    Presence {
        // TODO: strongly type these
        cursor_head: String,
        cursor_tail: Option<String>,
    },
}

#[record]
pub enum Event {
    /// edit a document
    Edit {
        user_id: UserId,

        /// the encoded update to this document
        update: DocumentUpdate,
    },

    /// update your presence in this document
    Presence {
        user_id: UserId,

        // TODO: strongly type these
        cursor_head: String,
        cursor_tail: Option<String>,
    },

    /// confirmation that the client is now subscribed to the document
    ///
    /// sent after the initial `Edit` containing the current document state
    /// has been sent. clients should wait for this event before sending
    /// `Presence` or `Edit` messages to avoid "not subscribed" errors.
    Subscribed,
}
