use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::routes;
use common::v1::types::application::Scope;
use common::v1::types::automod::{
    AutomodRule, AutomodRuleCreate, AutomodRuleTest, AutomodRuleTestRequest, AutomodRuleUpdate,
};
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::util::Changes;
use common::v1::types::{AuditLogEntryType, MessageSync, Permission, RoomId};
use http::StatusCode;
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;
use validator::Validate;

use crate::error::Result;
use crate::routes::util::Auth;
use crate::types::AutomodRuleId;
use crate::{routes2, Error, ServerState};

/// Automod rule list
#[handler(routes::automod_rule_list)]
async fn automod_rule_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::automod_rule_list::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let perms = srv.perms.for_room(auth.user.id, req.room_id).await?;
    perms.ensure(Permission::RoomEdit)?;

    let rules = s.data().automod_rule_list(req.room_id).await?;
    Ok(Json(rules))
}

/// Automod rule create
#[handler(routes::automod_rule_create)]
async fn automod_rule_create(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::automod_rule_create::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    req.rule.validate()?;

    let srv = s.services();
    let perms = srv.perms.for_room(auth.user.id, req.room_id).await?;
    perms.ensure(Permission::RoomEdit)?;

    let rule = s
        .data()
        .automod_rule_create(req.room_id, req.rule.clone())
        .await?;
    srv.automod.invalidate(req.room_id);

    let al = auth.audit_log(req.room_id);
    al.commit_success(AuditLogEntryType::AutomodRuleCreate {
        rule_id: rule.id,
        changes: Changes::new()
            .add("name", &rule.name)
            .add("enabled", &rule.enabled)
            .add("except_roles", &rule.except_roles)
            .add("except_channels", &rule.except_channels)
            .build(),
    })
    .await?;

    s.broadcast_room(
        req.room_id,
        auth.user.id,
        MessageSync::AutomodRuleCreate { rule: rule.clone() },
    )
    .await?;

    Ok((StatusCode::CREATED, Json(rule)))
}

/// Automod rule get
#[handler(routes::automod_rule_get)]
async fn automod_rule_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::automod_rule_get::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let perms = srv.perms.for_room(auth.user.id, req.room_id).await?;
    perms.ensure(Permission::RoomEdit)?;

    let rule = s.data().automod_rule_get(req.rule_id).await?;
    if rule.room_id != req.room_id {
        return Err(Error::ApiError(ApiError::from_code(
            ErrorCode::UnknownAutomodRule,
        )));
    }

    Ok(Json(rule))
}

/// Automod rule update
#[handler(routes::automod_rule_update)]
async fn automod_rule_update(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::automod_rule_update::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    req.rule.validate()?;

    let srv = s.services();
    let perms = srv.perms.for_room(auth.user.id, req.room_id).await?;
    perms.ensure(Permission::RoomEdit)?;

    let old = s.data().automod_rule_get(req.rule_id).await?;
    if old.room_id != req.room_id {
        return Err(Error::ApiError(ApiError::from_code(
            ErrorCode::UnknownAutomodRule,
        )));
    }

    let rule = s
        .data()
        .automod_rule_update(req.rule_id, req.rule.clone())
        .await?;
    srv.automod.invalidate(req.room_id);

    let al = auth.audit_log(req.room_id);
    al.commit_success(AuditLogEntryType::AutomodRuleUpdate {
        rule_id: req.rule_id,
        changes: Changes::new()
            .change("name", &old.name, &rule.name)
            .change("enabled", &old.enabled, &rule.enabled)
            .change("except_roles", &old.except_roles, &rule.except_roles)
            .change(
                "except_channels",
                &old.except_channels,
                &rule.except_channels,
            )
            .build(),
    })
    .await?;

    s.broadcast_room(
        req.room_id,
        auth.user.id,
        MessageSync::AutomodRuleUpdate { rule: rule.clone() },
    )
    .await?;

    Ok(Json(rule))
}

/// Automod rule delete
#[handler(routes::automod_rule_delete)]
async fn automod_rule_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::automod_rule_delete::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    let perms = srv.perms.for_room(auth.user.id, req.room_id).await?;
    perms.ensure(Permission::RoomEdit)?;

    let rule = s.data().automod_rule_get(req.rule_id).await?;
    if rule.room_id != req.room_id {
        return Err(Error::ApiError(ApiError::from_code(
            ErrorCode::UnknownAutomodRule,
        )));
    }

    s.data().automod_rule_delete(req.rule_id).await?;
    srv.automod.invalidate(req.room_id);

    let al = auth.audit_log(req.room_id);
    al.commit_success(AuditLogEntryType::AutomodRuleDelete {
        rule_id: req.rule_id,
        changes: Changes::new()
            .remove("name", &rule.name)
            .remove("enabled", &rule.enabled)
            .remove("except_roles", &rule.except_roles)
            .remove("except_channels", &rule.except_channels)
            .build(),
    })
    .await?;

    s.broadcast_room(
        req.room_id,
        auth.user.id,
        MessageSync::AutomodRuleDelete {
            rule_id: req.rule_id,
            room_id: req.room_id,
        },
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Automod rule test
#[handler(routes::automod_rule_test)]
async fn automod_rule_test(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::automod_rule_test::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let perms = srv.perms.for_room(auth.user.id, req.room_id).await?;
    perms.ensure(Permission::RoomEdit)?;

    let result = srv
        .automod
        .test_rules(req.room_id, &req.test.text, req.test.target)
        .await?;

    Ok(Json(result))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(automod_rule_list))
        .routes(routes2!(automod_rule_create))
        .routes(routes2!(automod_rule_get))
        .routes(routes2!(automod_rule_update))
        .routes(routes2!(automod_rule_delete))
        .routes(routes2!(automod_rule_test))
}
