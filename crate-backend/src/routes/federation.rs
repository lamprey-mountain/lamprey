use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use base64::Engine;
use common::v1::routes;
use common::v1::types::federation::{ServerKey, ServerKeys};
use common::v1::types::misc::ServerReq;
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;

use crate::error::Result;
use crate::routes::util::auth::Auth3;
use crate::routes::util::signing::sign_server_key;
use crate::{routes2, Error, ServerState};

/// Server keys get
///
/// Get the signing keys of a server
#[handler(routes::server_keys_get)]
async fn server_keys_get(
    State(s): State<Arc<ServerState>>,
    req: routes::server_keys_get::Request,
) -> Result<impl IntoResponse> {
    let local_hostname = s.config().hostname()?;

    let requested = match &req.hostname {
        ServerReq::ServerName(name) => name.as_str(),
        ServerReq::ServerHost => local_hostname,
        ServerReq::ServerClient => {
            // TODO: let servers see what we think their keys are?
            return Err(Error::Unimplemented);
        }
    };

    if requested != local_hostname {
        // TODO: use this server as a notary
        return Err(Error::Unimplemented);
    }

    let local_keys = s.services().federation.get_all_local_keys().await;

    let keys: Vec<ServerKey> = local_keys
        .iter()
        .map(|local_key| sign_server_key(local_key, local_hostname))
        .collect();

    Ok(Json(ServerKeys {
        hostname: local_hostname.to_owned(),
        keys,
    }))
}

/// Server user ensure (TODO)
///
/// Create a user representing a user on the requesting server
#[handler(routes::server_user_ensure)]
async fn server_user_ensure(
    State(_s): State<Arc<ServerState>>,
    _req: routes::server_user_ensure::Request,
    _auth: Auth3,
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
    _auth: Auth3,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Server ping
///
/// Check if a server is alive.
#[handler(routes::server_ping)]
async fn server_ping(
    State(s): State<Arc<ServerState>>,
    req: routes::server_ping::Request,
    auth: Auth3,
) -> Result<impl IntoResponse> {
    // TODO: allow local users to try to ping other servers
    let origin = auth.origin()?;

    let local_hostname = s.config().hostname()?;

    let requested = match &req.hostname {
        ServerReq::ServerName(name) => name.as_str(),
        ServerReq::ServerHost => local_hostname,
        ServerReq::ServerClient => origin.as_ref(),
    };

    if requested != local_hostname {
        // TODO: allow local users to ping remote servers
        // probably should return err if origin != local_hostname though
        return Err(Error::Unimplemented);
    }

    Ok(Json(routes::server_ping::PingResponse { federated: true }))
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
