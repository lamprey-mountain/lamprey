use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use common::v1::types::{
    audit_logs::{AuditLogChange, AuditLogEntry, AuditLogEntryType},
    sync::MessageSync,
    util::Changes,
    webhook::{Webhook, WebhookCreate, WebhookUpdate},
    AuditLogEntryId, Message, MessageCreate, Permission,
};
use serde_json::Value;
use utoipa_axum::{router::OpenApiRouter, routes};

use super::util::{Auth, HeaderReason};
use crate::{
    error::{Error, Result},
    types::{ChannelId, RoomId, WebhookId},
    ServerState,
};

/// Webhook create
#[utoipa::path(
    post,
    path = "/channel/{channel_id}/webhook",
    params(("channel_id", description = "channel id")),
    tags = ["webhook"],
    responses(
        (status = CREATED, body = Webhook, description = "Create webhook success"),
    )
)]
async fn webhook_create(
    Path(channel_id): Path<ChannelId>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<WebhookCreate>,
) -> Result<impl IntoResponse> {
    let channel = s.data().channel_get(channel_id).await?;
    let room_id = channel
        .room_id
        .ok_or(Error::BadRequest("Channel not in a room".to_string()))?;
    let perms = s.services().perms.for_room(auth_user.id, room_id).await?;
    perms.ensure(Permission::IntegrationsManage)?;

    let webhook = s
        .data()
        .webhook_create(channel_id, auth_user.id, json.clone())
        .await?;

    let audit_entry = AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth_user.id,
        session_id: None,
        reason,
        ty: AuditLogEntryType::WebhookCreate {
            webhook_id: webhook.id,
            changes: Changes::new()
                .add("name", &webhook.name)
                .add("channel_id", &webhook.channel_id)
                .build(),
        },
    };
    s.audit_log_append(audit_entry).await?;

    let sync_msg = MessageSync::WebhookCreate {
        webhook: webhook.clone(),
    };
    s.broadcast_room(room_id, auth_user.id, sync_msg).await?;

    Ok((StatusCode::CREATED, Json(webhook)))
}

/// Webhook list thread
#[utoipa::path(
    get,
    path = "/channel/{channel_id}/webhook",
    params(("channel_id", description = "channel id")),
    tags = ["webhook"],
    responses(
        (status = OK, body = Vec<Webhook>, description = "List webhooks success"),
    )
)]
async fn webhook_list_channel(
    Path(channel_id): Path<ChannelId>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let channel = s.data().channel_get(channel_id).await?;
    let room_id = channel
        .room_id
        .ok_or(Error::BadRequest("Channel not in a room".to_string()))?;
    let perms = s.services().perms.for_room(auth_user.id, room_id).await?;
    perms.ensure(Permission::IntegrationsManage)?;

    let webhooks = s.data().webhook_list_channel(channel_id).await?;

    Ok(Json(webhooks))
}

/// Webhook list room
#[utoipa::path(
    get,
    path = "/room/{room_id}/webhook",
    params(("room_id", description = "Room id")),
    tags = ["webhook"],
    responses(
        (status = OK, body = Vec<Webhook>, description = "List webhooks success"),
    )
)]
async fn webhook_list_room(
    Path(room_id): Path<RoomId>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let perms = s.services().perms.for_room(auth_user.id, room_id).await?;
    perms.ensure(Permission::IntegrationsManage)?;

    let webhooks = s.data().webhook_list_room(room_id).await?;

    Ok(Json(webhooks))
}

/// Webhook get
#[utoipa::path(
    get,
    path = "/webhook/{webhook_id}",
    params(("webhook_id", description = "Webhook id")),
    tags = ["webhook"],
    responses(
        (status = OK, body = Webhook, description = "Get webhook success"),
    )
)]
async fn webhook_get(
    Path(webhook_id): Path<WebhookId>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let webhook = s.data().webhook_get(webhook_id).await?;
    let room_id = webhook
        .room_id
        .ok_or(Error::BadRequest("Webhook not in a room".to_string()))?;
    let perms = s.services().perms.for_room(auth_user.id, room_id).await?;
    perms.ensure(Permission::IntegrationsManage)?;

    Ok(Json(webhook))
}

/// Webhook get with token
#[utoipa::path(
    get,
    path = "/webhook/{webhook_id}/{token}",
    params(
        ("webhook_id", description = "Webhook id"),
        ("token", description = "Webhook token")
    ),
    tags = ["webhook"],
    responses(
        (status = OK, body = Webhook, description = "Get webhook success"),
    )
)]
async fn webhook_get_with_token(
    Path((webhook_id, token)): Path<(WebhookId, String)>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let webhook = s.data().webhook_get_with_token(webhook_id, &token).await?;
    Ok(Json(webhook))
}

/// Webhook delete
#[utoipa::path(
    delete,
    path = "/webhook/{webhook_id}",
    params(("webhook_id", description = "Webhook id")),
    tags = ["webhook"],
    responses(
        (status = NO_CONTENT, description = "Delete webhook success"),
    )
)]
async fn webhook_delete(
    Path(webhook_id): Path<WebhookId>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    let webhook = s.data().webhook_get(webhook_id).await?;
    let room_id = webhook
        .room_id
        .ok_or(Error::BadRequest("Webhook not in a room".to_string()))?;
    let perms = s.services().perms.for_room(auth_user.id, room_id).await?;
    perms.ensure(Permission::IntegrationsManage)?;

    s.data().webhook_delete(webhook_id).await?;

    let audit_entry = AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth_user.id,
        session_id: None,
        reason,
        ty: AuditLogEntryType::WebhookDelete { webhook_id },
    };
    s.audit_log_append(audit_entry).await?;

    let sync_msg = MessageSync::WebhookDelete {
        webhook_id,
        room_id: webhook.room_id,
        channel_id: webhook.channel_id,
    };
    s.broadcast_room(room_id, auth_user.id, sync_msg).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Webhook delete with token
#[utoipa::path(
    delete,
    path = "/webhook/{webhook_id}/{token}",
    params(
        ("webhook_id", description = "Webhook id"),
        ("token", description = "Webhook token")
    ),
    tags = ["webhook"],
    responses(
        (status = NO_CONTENT, description = "Delete webhook success"),
    )
)]
async fn webhook_delete_with_token(
    Path((webhook_id, token)): Path<(WebhookId, String)>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    s.data()
        .webhook_delete_with_token(webhook_id, &token)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Webhook update
#[utoipa::path(
    patch,
    path = "/webhook/{webhook_id}",
    params(("webhook_id", description = "Webhook id")),
    tags = ["webhook"],
    responses(
        (status = OK, body = Webhook, description = "Update webhook success"),
    )
)]
async fn webhook_update(
    Path(webhook_id): Path<WebhookId>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<WebhookUpdate>,
) -> Result<impl IntoResponse> {
    let before_webhook = s.data().webhook_get(webhook_id).await?;
    let room_id = before_webhook
        .room_id
        .ok_or(Error::BadRequest("Webhook not in a room".to_string()))?;
    let perms = s.services().perms.for_room(auth_user.id, room_id).await?;
    perms.ensure(Permission::IntegrationsManage)?;

    let updated_webhook = s.data().webhook_update(webhook_id, json.clone()).await?;

    let mut changes = Changes::new()
        .change("name", &before_webhook.name, &updated_webhook.name)
        .change("avatar", &before_webhook.avatar, &updated_webhook.avatar)
        .change(
            "channel_id",
            &before_webhook.channel_id,
            &updated_webhook.channel_id,
        )
        .build();

    if json.rotate_token {
        changes.push(AuditLogChange {
            key: "token".to_string(),
            old: Value::Null,
            new: Value::Null,
        });
    }

    if !changes.is_empty() {
        let audit_entry = AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth_user.id,
            session_id: None,
            reason,
            ty: AuditLogEntryType::WebhookUpdate {
                webhook_id,
                changes,
            },
        };
        s.audit_log_append(audit_entry).await?;
    }

    let sync_msg = MessageSync::WebhookUpdate {
        webhook: updated_webhook.clone(),
    };
    s.broadcast_room(room_id, auth_user.id, sync_msg).await?;

    Ok(Json(updated_webhook))
}

/// Webhook update with token
#[utoipa::path(
    patch,
    path = "/webhook/{webhook_id}/{token}",
    params(
        ("webhook_id", description = "Webhook id"),
        ("token", description = "Webhook token")
    ),
    tags = ["webhook"],
    responses(
        (status = OK, body = Webhook, description = "Update webhook success"),
    )
)]
async fn webhook_update_with_token(
    Path((webhook_id, token)): Path<(WebhookId, String)>,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<WebhookUpdate>,
) -> Result<impl IntoResponse> {
    let updated_webhook = s
        .data()
        .webhook_update_with_token(webhook_id, &token, json)
        .await?;
    Ok(Json(updated_webhook))
}

/// Webhook execute
#[utoipa::path(
    post,
    path = "/webhook/{webhook_id}/{token}",
    params(
        ("webhook_id", description = "Webhook id"),
        ("token", description = "Webhook token")
    ),
    request_body = MessageCreate,
    tags = ["webhook"],
    responses(
        (status = CREATED, body = Message, description = "Execute webhook success, returns created message"),
    )
)]
async fn webhook_execute(
    Path((webhook_id, token)): Path<(WebhookId, String)>,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<MessageCreate>,
) -> Result<impl IntoResponse> {
    let webhook = s.data().webhook_get_with_token(webhook_id, &token).await?;

    let author_id = (*webhook.id).into();
    let channel_id = webhook.channel_id;

    let srv = s.services();
    let message = srv
        .messages
        .create(channel_id, author_id, None, None, json)
        .await?;

    Ok((StatusCode::CREATED, Json(message)))
}

/// Webhook execute discord (TODO)
#[utoipa::path(
    post,
    path = "/webhook/{webhook_id}/{token}/discord",
    params(
        ("webhook_id", description = "Webhook id"),
        ("token", description = "Webhook token")
    ),
    tags = ["webhook"],
    responses(
        (status = NO_CONTENT, description = "Execute webhook success"),
    )
)]
async fn webhook_execute_discord(
    Path((_webhook_id, _token)): Path<(WebhookId, String)>,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Webhook execute github (TODO)
#[utoipa::path(
    post,
    path = "/webhook/{webhook_id}/{token}/github",
    params(
        ("webhook_id", description = "Webhook id"),
        ("token", description = "Webhook token")
    ),
    tags = ["webhook"],
    responses(
        (status = NO_CONTENT, description = "Execute webhook success"),
    )
)]
async fn webhook_execute_github(
    Path((_webhook_id, _token)): Path<(WebhookId, String)>,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Webhook execute slack (TODO)
#[utoipa::path(
    post,
    path = "/webhook/{webhook_id}/{token}/slack",
    params(
        ("webhook_id", description = "Webhook id"),
        ("token", description = "Webhook token")
    ),
    tags = ["webhook"],
    responses(
        (status = NO_CONTENT, description = "Execute webhook success"),
    )
)]
async fn webhook_execute_slack(
    Path((_webhook_id, _token)): Path<(WebhookId, String)>,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(webhook_create))
        .routes(routes!(webhook_list_channel))
        .routes(routes!(webhook_list_room))
        .routes(routes!(webhook_get))
        .routes(routes!(webhook_get_with_token))
        .routes(routes!(webhook_delete))
        .routes(routes!(webhook_delete_with_token))
        .routes(routes!(webhook_update))
        .routes(routes!(webhook_update_with_token))
        .routes(routes!(webhook_execute))
        .routes(routes!(webhook_execute_discord))
        .routes(routes!(webhook_execute_github))
        .routes(routes!(webhook_execute_slack))
}
