use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::Json;
use common::v1::types::application::Connection;
use common::v1::types::misc::UserIdReq;
use common::v1::types::user_connection::{ConnectionMetadata, ConnectionPatch};
use common::v1::types::{
    ApplicationId, AuditLogEntryType, MessageSync, PaginationQuery, PaginationResponse,
};
use http::StatusCode;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::routes::util::Auth;
use crate::{Error, Result, ServerState};

/// Get user connections
#[utoipa::path(
    get,
    path = "/user/{user_id}/connection",
    params(
        ("user_id", description = "User id"),
        PaginationQuery<ApplicationId>
    ),
    tags = ["user_connection"],
    responses(
        (status = OK, body = PaginationResponse<Connection>, description = "success"),
    )
)]
async fn user_connection_list(
    Path(user_id): Path<UserIdReq>,
    Query(paginate): Query<PaginationQuery<ApplicationId>>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };

    if auth.user.id != target_user_id {
        return Err(Error::MissingPermissions);
    }

    let connections = s.data().connection_list(target_user_id, paginate).await?;
    Ok(Json(connections))
}

/// User connection update (TODO)
#[utoipa::path(
    patch,
    path = "/user/{user_id}/connection/{app_id}",
    params(
        ("user_id", description = "User id"),
        ("app_id", description = "Application id"),
    ),
    tags = ["user_connection"],
    responses(
        (status = OK, body = Connection, description = "success"),
    )
)]
async fn user_connection_update(
    Path((_user_id, _app_id)): Path<(UserIdReq, ApplicationId)>,
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_patch): Json<ConnectionPatch>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// User connection delete
#[utoipa::path(
    delete,
    path = "/user/{user_id}/connection/{app_id}",
    params(
        ("user_id", description = "User id"),
        ("app_id", description = "Application id")
    ),
    tags = ["user_connection"],
    responses((status = NO_CONTENT, description = "success")),
)]
async fn user_connection_delete(
    Path((target_user_id, app_id)): Path<(UserIdReq, ApplicationId)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };

    if auth.user.id != target_user_id {
        return Err(Error::MissingPermissions);
    }

    s.data().connection_delete(target_user_id, app_id).await?;

    s.broadcast(MessageSync::ConnectionDelete {
        user_id: target_user_id,
        app_id,
    })?;
    let al = auth.audit_log(target_user_id.into_inner().into());
    al.commit_success(AuditLogEntryType::ConnectionDelete {
        application_id: app_id,
    })
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// User connection metadata get (TODO)
#[utoipa::path(
    get,
    path = "/user/@self/app/{app_id}/connection-metadata",
    params(
        ("app_id", description = "Application id"),
    ),
    tags = ["user_connection"],
    responses(
        (status = OK, body = ConnectionMetadata, description = "success"),
    )
)]
async fn user_app_connection_metadata_get(
    Path(_app_id): Path<ApplicationId>,
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// User connection metadata put (TODO)
#[utoipa::path(
    put,
    path = "/user/@self/app/{app_id}/connection-metadata",
    params(
        ("app_id", description = "Application id"),
    ),
    tags = ["user_connection"],
    responses(
        (status = OK, body = ConnectionMetadata, description = "success"),
    )
)]
async fn user_app_connection_metadata_put(
    Path(_app_id): Path<ApplicationId>,
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_metadata): Json<ConnectionMetadata>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(user_connection_list))
        .routes(routes!(user_connection_update))
        .routes(routes!(user_connection_delete))
        .routes(routes!(user_app_connection_metadata_get))
        .routes(routes!(user_app_connection_metadata_put))
}
