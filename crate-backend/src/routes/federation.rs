use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use common::v1::types::{
    federation::{ServerKeys, ServerUserCreate, ServerUserCreateRequest},
    misc::ServerReq,
};
use utoipa_axum::{router::OpenApiRouter, routes};

use super::util::Auth;

use crate::error::Result;
use crate::{Error, ServerState};

/// Server keys get (TODO)
///
/// Get the signing keys of a server
#[utoipa::path(
    post,
    path = "/server/{hostname}",
    params(("hostname", description = "Server hostname")),
    tags = ["federation"],
    responses(
        (status = OK, body = ServerKeys, description = "ok"),
    )
)]
async fn server_keys_get(
    Path(hostname): Path<ServerReq>,
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let _hostname = match hostname {
        ServerReq::ServerHost => todo!(),
        ServerReq::ServerClient => todo!(),
        ServerReq::ServerFqdn(host) => host,
    };

    Ok(Error::Unimplemented)
}

/// Server user ensure (TODO)
///
/// Create a user representing a user on the requesting server
#[utoipa::path(
    post,
    path = "/server/{hostname}/user",
    params(("hostname", description = "Server hostname")),
    tags = ["federation"],
    responses(
        (status = OK, body = ServerUserCreate, description = "ok"),
    )
)]
async fn server_user_ensure(
    Path(hostname): Path<ServerReq>,
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<ServerUserCreateRequest>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let _hostname: String = match hostname {
        ServerReq::ServerHost => {
            return Err(Error::BadStatic("can only create user on your own server"))
        }
        ServerReq::ServerClient => todo!("valid"),
        ServerReq::ServerFqdn(_) => todo!("only valid if fqdn == client"),
    };

    Ok(Error::Unimplemented)
}

/// Server sync handle (TODO)
///
/// Handle MessageSync events. used to proxy events to connected clients.
// NOTE: in the future, i probably want to have a local cache of remote data too
#[utoipa::path(
    post,
    path = "/server/{hostname}/sync",
    params(("hostname", description = "Server hostname")),
    tags = ["federation"],
    responses((status = ACCEPTED, description = "ok")),
)]
async fn server_sync_handle(
    Path(hostname): Path<ServerReq>,
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<ServerUserCreateRequest>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let _hostname: String = match hostname {
        ServerReq::ServerHost => todo!("valid"),
        ServerReq::ServerClient => return Err(Error::BadStatic("can only sync for this server")),
        ServerReq::ServerFqdn(_) => todo!("only valid if fqdn == remote"),
    };

    Ok(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(server_keys_get))
        .routes(routes!(server_user_ensure))
        .routes(routes!(server_sync_handle))
}
