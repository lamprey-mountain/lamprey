use crate::prelude::*;

use axum::extract::FromRequest;
use common::util::routes::Endpoint;

use crate::util::Auth;

/// the current state for a request
///
/// can be used as an axum extractor
pub struct Req<E: Endpoint> {
    pub auth: Auth,
    pub body: E::Request,

    pub globals: Globals,

    /// resolved media
    media: (),

    reason: Option<String>,
    // headers: (),
    // audit_txn_slot: AuditTxnSlot,
}

impl<E> FromRequest<Globals> for Req<E>
where
    E: Endpoint + Send,
    E::Request: Send,
{
    type Rejection = Error;

    async fn from_request(req: axum::extract::Request, state: &Globals) -> Result<Self> {
        todo!()
    }
}

impl<E: Endpoint> Req<E> {
    pub fn services(&self) -> ! {
        todo!()
    }

    // pub fn get_media(&self, media_ref: &MediaReference) -> &Media {
    //     todo!()
    // }

    // /// begin an audit log transaction
    // #[must_use = "must call commit() to save a successful audit log entry"]
    // pub async fn begin_audit_log(
    //     &self,
    //     room_id: RoomId,
    //     ty: AuditLogEntryType,
    // ) -> Result<AuditTxnHandle> {
    //     todo!()
    // }
}

// // see crate-backend-services/src/services/permissions.rs
// pub struct Requirements;

// // resolved permissions?
// pub struct Permissions;

// impl Req<E: Endpoint> {
//     /// enforce a set of requirements
//     pub fn enforce(&self, requirements: Requirements) -> Result<Permissions> {
//         todo!()
//     }
// }
