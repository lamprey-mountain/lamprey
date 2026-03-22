#![allow(unused)] // TEMP

use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use common::v1::routes;
use common::v1::types::application::Scope;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::federation::{ServerKeys, ServerUserCreate, ServerUserCreateRequest};
use common::v1::types::misc::ServerReq;
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;

use crate::error::Result;
use crate::routes::util::Auth;
use crate::{routes2, Error, ServerState};

/// Server keys get (TODO)
///
/// Get the signing keys of a server
#[handler(routes::server_keys_get)]
async fn server_keys_get(
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::server_keys_get::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    Ok(Error::Unimplemented)
}

/// Server user ensure (TODO)
///
/// Create a user representing a user on the requesting server
#[handler(routes::server_user_ensure)]
async fn server_user_ensure(
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::server_user_ensure::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    Ok(Error::Unimplemented)
}

/// Server sync handle (TODO)
///
/// Handle MessageSync events. used to proxy events to connected clients.
// NOTE: in the future, i probably want to have a local cache of remote data too
#[handler(routes::server_sync_handle)]
async fn server_sync_handle(
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::server_sync_handle::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    Ok(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(server_keys_get))
        .routes(routes2!(server_user_ensure))
        .routes(routes2!(server_sync_handle))
}
