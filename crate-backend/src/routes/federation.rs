use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::routes;
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;

use crate::error::Result;
use crate::routes::util::auth::Auth2;
use crate::{routes2, Error, ServerState};

/// Server keys get (TODO)
///
/// Get the signing keys of a server
#[handler(routes::server_keys_get)]
async fn server_keys_get(
    State(_s): State<Arc<ServerState>>,
    _req: routes::server_keys_get::Request,
    _auth: Auth2,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Server user ensure (TODO)
///
/// Create a user representing a user on the requesting server
#[handler(routes::server_user_ensure)]
async fn server_user_ensure(
    State(_s): State<Arc<ServerState>>,
    _req: routes::server_user_ensure::Request,
    _auth: Auth2,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Server sync handle (TODO)
///
/// Handle MessageSync events. used to proxy events to connected clients.
// NOTE: in the future, i probably want to have a local cache of remote data too
#[handler(routes::server_sync_handle)]
async fn server_sync_handle(
    State(_s): State<Arc<ServerState>>,
    _req: routes::server_sync_handle::Request,
    _auth: Auth2,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Server ping
///
/// Check if a server is alive.
#[handler(routes::server_ping)]
async fn server_ping(
    State(_s): State<Arc<ServerState>>,
    _req: routes::server_ping::Request,
    auth: Auth2,
) -> Result<impl IntoResponse> {
    auth.origin()?;

    Ok(Json(routes::server_ping::Response {
        body: routes::server_ping::PingResponse { ok: true },
    }))
}

pub fn routes(s: Arc<ServerState>) -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(server_keys_get))
        .routes(routes2!(server_user_ensure))
        .routes(routes2!(server_sync_handle))
        .routes(routes2!(server_ping))
        .layer(axum::middleware::from_fn_with_state(
            s,
            crate::routes::util::federation_auth_middleware,
        ))
}
