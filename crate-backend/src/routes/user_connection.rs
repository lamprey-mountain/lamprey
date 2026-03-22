use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::routes;
use common::v1::types::misc::UserIdReq;
use common::v1::types::{AuditLogEntryType, MessageSync};
use http::StatusCode;
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;

use crate::error::Result;
use crate::routes::util::Auth;
use crate::{routes2, Error, ServerState};

/// Get user connections
#[handler(routes::user_connection_list)]
async fn user_connection_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::user_connection_list::Request,
) -> Result<impl IntoResponse> {
    let target_user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };

    if auth.user.id != target_user_id {
        return Err(Error::MissingPermissions);
    }

    let connections = s
        .data()
        .connection_list(target_user_id, req.pagination)
        .await?;
    Ok(Json(connections))
}

/// User connection update (TODO)
#[handler(routes::user_connection_update)]
async fn user_connection_update(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::user_connection_update::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// User connection delete
#[handler(routes::user_connection_delete)]
async fn user_connection_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::user_connection_delete::Request,
) -> Result<impl IntoResponse> {
    let target_user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };

    if auth.user.id != target_user_id {
        return Err(Error::MissingPermissions);
    }

    s.data()
        .connection_delete(target_user_id, req.app_id)
        .await?;

    s.broadcast(MessageSync::ConnectionDelete {
        user_id: target_user_id,
        app_id: req.app_id,
    })?;
    let al = auth.audit_log(target_user_id.into_inner().into());
    al.commit_success(AuditLogEntryType::ConnectionDelete {
        application_id: req.app_id,
    })
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// User connection metadata get (TODO)
#[handler(routes::user_connection_metadata_get)]
async fn user_connection_metadata_get(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::user_connection_metadata_get::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// User connection metadata put (TODO)
#[handler(routes::user_connection_metadata_put)]
async fn user_connection_metadata_put(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::user_connection_metadata_put::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(user_connection_list))
        .routes(routes2!(user_connection_update))
        .routes(routes2!(user_connection_delete))
        .routes(routes2!(user_connection_metadata_get))
        .routes(routes2!(user_connection_metadata_put))
}
