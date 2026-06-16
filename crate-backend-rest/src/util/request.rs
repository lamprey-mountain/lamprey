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

    /// resolved media
    media: (),

    reason: Option<String>,
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

// impl Req<E: Endpoint> {
//     pub fn a(&self) -> () {
//         todo!()
//     }
// }
