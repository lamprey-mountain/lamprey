use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use common::v1::types::automod::{AutomodRule, AutomodRuleCreate, AutomodRuleUpdate};
use utoipa_axum::{router::OpenApiRouter, routes};

use super::util::Auth2;
use crate::{
    error::{Error, Result},
    types::{AutomodRuleId, PaginationResponse, RoomId},
    ServerState,
};

/// Automod rule list (TODO)
#[utoipa::path(
    get,
    path = "/room/{room_id}/automod",
    params(("room_id", description = "Room id")),
    tags = ["automod"],
    responses(
        (status = OK, body = PaginationResponse<AutomodRule>, description = "List automod rules success"),
    )
)]
async fn automod_rule_list(
    Path(_room_id): Path<RoomId>,
    _auth: Auth2,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Automod rule create (TODO)
#[utoipa::path(
    post,
    path = "/room/{room_id}/automod",
    params(("room_id", description = "Room id")),
    tags = ["automod"],
    responses(
        (status = CREATED, body = AutomodRule, description = "Create automod rule success"),
    )
)]
async fn automod_rule_create(
    Path(_room_id): Path<RoomId>,
    _auth: Auth2,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<AutomodRuleCreate>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Automod rule get (TODO)
#[utoipa::path(
    get,
    path = "/room/{room_id}/automod/{rule_id}",
    params(
        ("room_id", description = "Room id"),
        ("rule_id", description = "Rule id")
    ),
    tags = ["automod"],
    responses(
        (status = OK, body = AutomodRule, description = "Get automod rule success"),
    )
)]
async fn automod_rule_get(
    Path((_room_id, _rule_id)): Path<(RoomId, AutomodRuleId)>,
    _auth: Auth2,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Automod rule update (TODO)
#[utoipa::path(
    patch,
    path = "/room/{room_id}/automod/{rule_id}",
    params(
        ("room_id", description = "Room id"),
        ("rule_id", description = "Rule id")
    ),
    tags = ["automod"],
    responses(
        (status = OK, body = AutomodRule, description = "Update automod rule success"),
    )
)]
async fn automod_rule_update(
    Path((_room_id, _rule_id)): Path<(RoomId, AutomodRuleId)>,
    _auth: Auth2,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<AutomodRuleUpdate>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Automod rule delete (TODO)
#[utoipa::path(
    delete,
    path = "/room/{room_id}/automod/{rule_id}",
    params(
        ("room_id", description = "Room id"),
        ("rule_id", description = "Rule id")
    ),
    tags = ["automod"],
    responses(
        (status = NO_CONTENT, description = "Delete automod rule success"),
    )
)]
async fn automod_rule_delete(
    Path((_room_id, _rule_id)): Path<(RoomId, AutomodRuleId)>,
    _auth: Auth2,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Automod rule test (TODO)
#[utoipa::path(
    post,
    path = "/room/{room_id}/automod/test",
    params(
        ("room_id", description = "Room id"),
        ("rule_id", description = "Rule id")
    ),
    tags = ["automod"],
    responses(
        (status = OK, description = "Text was scanned"),
    )
)]
async fn automod_rule_test(
    Path((_room_id,)): Path<(RoomId,)>,
    _auth: Auth2,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(automod_rule_list))
        .routes(routes!(automod_rule_create))
        .routes(routes!(automod_rule_get))
        .routes(routes!(automod_rule_update))
        .routes(routes!(automod_rule_delete))
        .routes(routes!(automod_rule_test))
}
