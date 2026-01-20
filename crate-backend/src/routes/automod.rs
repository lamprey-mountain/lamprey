use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use common::v1::types::automod::{
    AutomodRule, AutomodRuleCreate, AutomodRuleTest, AutomodRuleTestRequest, AutomodRuleUpdate,
};
use http::StatusCode;
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use super::util::Auth;
use crate::{
    error::{Error, Result},
    types::{AutomodRuleId, RoomId},
    ServerState,
};

use common::v1::types::util::Changes;
use common::v1::types::{
    AuditLogEntry, AuditLogEntryId, AuditLogEntryType, MessageSync, Permission,
};

/// Automod rule list
#[utoipa::path(
    get,
    path = "/room/{room_id}/automod/rule",
    params(("room_id", description = "Room id")),
    tags = ["automod"],
    responses(
        (status = OK, body = Vec<AutomodRule>, description = "List automod rules success"),
    )
)]
async fn automod_rule_list(
    Path(room_id): Path<RoomId>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_room(auth.user.id, room_id).await?;
    perms.ensure(Permission::RoomManage)?;

    let rules = s.data().automod_rule_list(room_id).await?;
    Ok(Json(rules))
}

/// Automod rule create
#[utoipa::path(
    post,
    path = "/room/{room_id}/automod/rule",
    params(("room_id", description = "Room id")),
    tags = ["automod"],
    responses(
        (status = CREATED, body = AutomodRule, description = "Create automod rule success"),
    )
)]
async fn automod_rule_create(
    Path(room_id): Path<RoomId>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<AutomodRuleCreate>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    json.validate()?;

    let srv = s.services();
    let perms = srv.perms.for_room(auth.user.id, room_id).await?;
    perms.ensure(Permission::RoomManage)?;

    let rule = s.data().automod_rule_create(room_id, json.clone()).await?;
    srv.automod.invalidate(room_id);

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth.user.id,
        session_id: Some(auth.session.id),
        reason: None,
        ty: AuditLogEntryType::AutomodRuleCreate {
            rule_id: rule.id,
            changes: Changes::new()
                .add("name", &rule.name)
                .add("enabled", &rule.enabled)
                // TODO: log trigger and actions
                // .add("trigger", &rule.trigger)
                // .add("actions", &rule.actions)
                .add("except_roles", &rule.except_roles)
                .add("except_channels", &rule.except_channels)
                .build(),
        },
    })
    .await?;

    s.broadcast_room(
        room_id,
        auth.user.id,
        MessageSync::AutomodRuleCreate { rule: rule.clone() },
    )
    .await?;

    Ok((StatusCode::CREATED, Json(rule)))
}

/// Automod rule get
#[utoipa::path(
    get,
    path = "/room/{room_id}/automod/rule/{rule_id}",
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
    Path((room_id, rule_id)): Path<(RoomId, AutomodRuleId)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_room(auth.user.id, room_id).await?;
    perms.ensure(Permission::RoomManage)?;

    let rule = s.data().automod_rule_get(rule_id).await?;
    if rule.room_id != room_id {
        return Err(Error::NotFound);
    }

    Ok(Json(rule))
}

/// Automod rule update
#[utoipa::path(
    patch,
    path = "/room/{room_id}/automod/rule/{rule_id}",
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
    Path((room_id, rule_id)): Path<(RoomId, AutomodRuleId)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<AutomodRuleUpdate>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    json.validate()?;

    let srv = s.services();
    let perms = srv.perms.for_room(auth.user.id, room_id).await?;
    perms.ensure(Permission::RoomManage)?;

    let old = s.data().automod_rule_get(rule_id).await?;
    if old.room_id != room_id {
        return Err(Error::NotFound);
    }

    let rule = s.data().automod_rule_update(rule_id, json.clone()).await?;
    srv.automod.invalidate(room_id);

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth.user.id,
        session_id: Some(auth.session.id),
        reason: None,
        ty: AuditLogEntryType::AutomodRuleUpdate {
            rule_id,
            changes: Changes::new()
                .change("name", &old.name, &rule.name)
                .change("enabled", &old.enabled, &rule.enabled)
                // TODO: log trigger and actions
                // .change("trigger", &old.trigger, &rule.trigger)
                // .change("actions", &old.actions, &rule.actions)
                .change("except_roles", &old.except_roles, &rule.except_roles)
                .change(
                    "except_channels",
                    &old.except_channels,
                    &rule.except_channels,
                )
                .build(),
        },
    })
    .await?;

    s.broadcast_room(
        room_id,
        auth.user.id,
        MessageSync::AutomodRuleUpdate { rule: rule.clone() },
    )
    .await?;

    Ok(Json(rule))
}

/// Automod rule delete
#[utoipa::path(
    delete,
    path = "/room/{room_id}/automod/rule/{rule_id}",
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
    Path((room_id, rule_id)): Path<(RoomId, AutomodRuleId)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let srv = s.services();
    let perms = srv.perms.for_room(auth.user.id, room_id).await?;
    perms.ensure(Permission::RoomManage)?;

    let rule = s.data().automod_rule_get(rule_id).await?;
    if rule.room_id != room_id {
        return Err(Error::NotFound);
    }

    s.data().automod_rule_delete(rule_id).await?;
    srv.automod.invalidate(room_id);

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth.user.id,
        session_id: Some(auth.session.id),
        reason: None,
        ty: AuditLogEntryType::AutomodRuleDelete {
            rule_id,
            changes: Changes::new()
                .remove("name", &rule.name)
                .remove("enabled", &rule.enabled)
                // TODO: log trigger and actions
                // .remove("trigger", &rule.trigger)
                // .remove("actions", &rule.actions)
                .remove("except_roles", &rule.except_roles)
                .remove("except_channels", &rule.except_channels)
                .build(),
        },
    })
    .await?;

    s.broadcast_room(
        room_id,
        auth.user.id,
        MessageSync::AutomodRuleDelete { rule_id, room_id },
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Automod rule test
#[utoipa::path(
    post,
    path = "/room/{room_id}/automod/rule/test",
    params(
        ("room_id", description = "Room id"),
    ),
    tags = ["automod"],
    request_body = AutomodRuleTestRequest,
    responses(
        (status = OK, body = AutomodRuleTest, description = "Test automod rules success"),
    )
)]
async fn automod_rule_test(
    Path(room_id): Path<RoomId>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<AutomodRuleTestRequest>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_room(auth.user.id, room_id).await?;
    perms.ensure(Permission::RoomManage)?;

    let result = srv
        .automod
        .test_rules(room_id, &json.text, json.target)
        .await?;

    Ok(Json(result))
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
