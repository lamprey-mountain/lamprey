use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::routes;
use common::v1::types::federation::{
    Hostname, ServerConnectResponse, ServerKey, ServerKeys, ServerPingResponse, ServerSyncResponse,
};
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

/// Server connect
#[handler(routes::server_connect)]
async fn server_connect(
    State(s): State<Arc<ServerState>>,
    req: routes::server_connect::Request,
    auth: Auth3,
) -> Result<impl IntoResponse> {
    let origin = auth.origin()?;

    let local_hostname = s.config().hostname()?;

    let target = match &req.hostname {
        ServerReq::ServerName(name) => name.as_str(),
        ServerReq::ServerHost => local_hostname,
        ServerReq::ServerClient => {
            return Err(Error::BadRequest("invalid target hostname".to_string()));
        }
    };

    if target != local_hostname {
        return Err(Error::BadRequest("wrong target hostname".to_string()));
    }

    // register server by connecting back to establish mutual sync
    s.services().federation.connect(origin.clone()).await?;

    Ok(Json(ServerConnectResponse {}))
}

/// Server sync handle
///
/// Handle MessageSync events. used to proxy events to connected clients.
#[handler(routes::server_sync_handle)]
async fn server_sync_handle(
    State(s): State<Arc<ServerState>>,
    req: routes::server_sync_handle::Request,
    auth: Auth3,
) -> Result<impl IntoResponse> {
    let _origin = auth.origin()?;
    // TODO: return a useful http response
    s.services().federation.handle_sync(req.sync).await?;
    Ok(Json(ServerSyncResponse {}))
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
    let origin = auth.origin().ok();
    let is_federated = origin.is_some();

    let local_hostname = s.config().hostname()?;

    let target = match &req.hostname {
        ServerReq::ServerName(name) => name.as_str(),
        ServerReq::ServerHost => local_hostname,
        ServerReq::ServerClient => {
            if let Some(origin) = origin {
                origin.as_ref()
            } else {
                return Err(Error::BadRequest(
                    "ServerClient requires federation auth".to_string(),
                ));
            }
        }
    };

    if target != local_hostname {
        // NOTE: federated should always be true since this is a server -> server ping (client -> server -> server)
        let federated = s
            .services()
            .federation
            .ping(Hostname(target.to_string()))
            .await?;
        return Ok(Json(ServerPingResponse { federated }));
    }

    Ok(Json(ServerPingResponse {
        federated: is_federated,
    }))
}

pub fn routes(s: Arc<ServerState>) -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(server_keys_get))
        .routes(routes2!(server_connect))
        .routes(routes2!(server_sync_handle))
        .routes(routes2!(server_ping))
        .layer(axum::middleware::from_fn_with_state(
            s,
            crate::routes::util::federation_auth_middleware,
        ))
}
