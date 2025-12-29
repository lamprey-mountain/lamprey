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
async fn list_rules(
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
async fn create_rule(
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
async fn get_rule(
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
async fn update_rule(
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
async fn delete_rule(
    Path((_room_id, _rule_id)): Path<(RoomId, AutomodRuleId)>,
    _auth: Auth2,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(list_rules))
        .routes(routes!(create_rule))
        .routes(routes!(get_rule))
        .routes(routes!(update_rule))
        .routes(routes!(delete_rule))
}
