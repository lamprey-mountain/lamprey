use crate::prelude::*;

use axum::extract::FromRequest;
use common::util::routes::Endpoint;
use kerosene_core::types::auth::Identity;
use lamprey_backend_services::services::Services;

/// the current state for a request
///
/// can be used as an axum extractor
pub struct Req<E: Endpoint> {
    inner: E::Request,

    /// a handle to global state
    globals: Globals,

    /// the identity of who is making this request
    identity: Identity,

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
    type Rejection = ServerError;

    async fn from_request(req: axum::extract::Request, state: &Globals) -> ServerResult<Self> {
        todo!()
    }
}

impl<E: Endpoint> Req<E> {
    #[inline]
    pub fn globals(&self) -> Globals {
        self.globals.clone()
    }

    #[inline]
    pub fn services(&self) -> Arc<Services> {
        self.globals.services()
    }

    #[inline]
    pub fn identity(&self) -> &Identity {
        &self.identity
    }

    #[inline]
    pub fn inner(&self) -> &E::Request {
        &self.inner
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
