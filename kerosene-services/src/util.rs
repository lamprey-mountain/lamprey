use crate::prelude::*;

use common::v2::types::RoomId;
use lamprey_backend_core::types::auth::Identity;

// TODO: impl compat for Auth4

/// current state of a request
// NOTE: similar to auth or universal extractor?
pub struct RequestState<T> {
    /// the request body itself
    body: T,

    /// resolved media
    media: (),

    /// identity of the actor creating this request
    identity: Identity,

    // merge reason, etc?
    // reason: Option<String>,
    headers: (),
    // audit_txn_slot: AuditTxnSlot,
}

pub struct AuditTxn {
    // ...
}

pub struct AuditTxnHandle {
    // ...
}

impl AuditTxnHandle {
    /// commit this audit log transaction
    ///
    /// this saves the audit log entry to the database and broadcasts an `AuditLogEntryCreate` sync message
    pub async fn commit(self) -> Result<()> {
        todo!()
    }
}

impl<T> RequestState<T> {
    /// get the extracted body
    pub fn take_body(self) -> (T, RequestState<()>) {
        (self.body, todo!())
    }

    pub fn body(&self) -> &T {
        &self.body
    }

    pub fn identity(&self) -> &Identity {
        &self.identity
    }

    pub fn begin_audit_txn(&self, room_id: RoomId) -> AuditTxnHandle {
        todo!()
    }

    // pub fn media(&self, media_ref: &MediaReference) -> &Media {
    //     todo!()
    // }
}
