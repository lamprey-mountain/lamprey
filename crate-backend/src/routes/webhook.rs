use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use common::v1::types::{
    webhook::{Webhook, WebhookCreate, WebhookUpdate},
    Message, MessageCreate, Permission,
};
use utoipa_axum::{router::OpenApiRouter, routes};

use super::util::Auth;
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
async fn create_webhook(
    Path(channel_id): Path<ChannelId>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
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
        .webhook_create(channel_id, auth_user.id, json)
        .await?;

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
async fn list_webhooks_thread(
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
async fn list_webhooks_room(
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
async fn get_webhook(
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
async fn get_webhook_with_token(
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
async fn delete_webhook(
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

    s.data().webhook_delete(webhook_id).await?;

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
async fn delete_webhook_with_token(
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
async fn update_webhook(
    Path(webhook_id): Path<WebhookId>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<WebhookUpdate>,
) -> Result<impl IntoResponse> {
    let webhook = s.data().webhook_get(webhook_id).await?;
    let room_id = webhook
        .room_id
        .ok_or(Error::BadRequest("Webhook not in a room".to_string()))?;
    let perms = s.services().perms.for_room(auth_user.id, room_id).await?;
    perms.ensure(Permission::IntegrationsManage)?;

    let updated_webhook = s.data().webhook_update(webhook_id, json).await?;

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
async fn update_webhook_with_token(
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
async fn execute_webhook(
    Path((webhook_id, token)): Path<(WebhookId, String)>,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<MessageCreate>,
) -> Result<impl IntoResponse> {
    let webhook = s.data().webhook_get_with_token(webhook_id, &token).await?;

    let author_id = (*webhook.id).into();
    let channel_id = webhook.thread_id;

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
async fn execute_webhook_discord(
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
async fn execute_webhook_github(
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
async fn execute_webhook_slack(
    Path((_webhook_id, _token)): Path<(WebhookId, String)>,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(create_webhook))
        .routes(routes!(list_webhooks_thread))
        .routes(routes!(list_webhooks_room))
        .routes(routes!(get_webhook))
        .routes(routes!(get_webhook_with_token))
        .routes(routes!(delete_webhook))
        .routes(routes!(delete_webhook_with_token))
        .routes(routes!(update_webhook))
        .routes(routes!(update_webhook_with_token))
        .routes(routes!(execute_webhook))
        .routes(routes!(execute_webhook_discord))
        .routes(routes!(execute_webhook_github))
        .routes(routes!(execute_webhook_slack))
}
