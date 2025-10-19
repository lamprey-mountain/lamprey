use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use common::v1::types::webhook::{Webhook, WebhookCreate, WebhookUpdate};
use utoipa_axum::{router::OpenApiRouter, routes};

use super::util::Auth;
use crate::{
    error::{Error, Result},
    types::{ChannelId, RoomId, WebhookId},
    ServerState,
};

/// Webhook create (TODO)
#[utoipa::path(
    post,
    path = "/thread/{thread_id}/webhook",
    params(("thread_id", description = "Thread id")),
    tags = ["webhook"],
    responses(
        (status = CREATED, body = Webhook, description = "Create webhook success"),
    )
)]
async fn create_webhook(
    Path(_thread_id): Path<ChannelId>,
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<WebhookCreate>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Webhook list thread (TODO)
#[utoipa::path(
    get,
    path = "/thread/{thread_id}/webhook",
    params(("thread_id", description = "Thread id")),
    tags = ["webhook"],
    responses(
        (status = OK, body = Vec<Webhook>, description = "List webhooks success"),
    )
)]
async fn list_webhooks_thread(
    Path(_thread_id): Path<ChannelId>,
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Webhook list room (TODO)
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
    Path(_room_id): Path<RoomId>,
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Webhook get (TODO)
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
    Path(_webhook_id): Path<WebhookId>,
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Webhook get with token (TODO)
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
    Path((_webhook_id, _token)): Path<(WebhookId, String)>,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Webhook delete (TODO)
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
    Path(_webhook_id): Path<WebhookId>,
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Webhook delete with token (TODO)
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
    Path((_webhook_id, _token)): Path<(WebhookId, String)>,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Webhook update (TODO)
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
    Path(_webhook_id): Path<WebhookId>,
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<WebhookUpdate>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Webhook update with token (TODO)
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
    Path((_webhook_id, _token)): Path<(WebhookId, String)>,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<WebhookUpdate>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Webhook execute (TODO)
#[utoipa::path(
    post,
    path = "/webhook/{webhook_id}/{token}",
    params(
        ("webhook_id", description = "Webhook id"),
        ("token", description = "Webhook token")
    ),
    tags = ["webhook"],
    responses(
        (status = NO_CONTENT, description = "Execute webhook success"),
    )
)]
async fn execute_webhook(
    Path((_webhook_id, _token)): Path<(WebhookId, String)>,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
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
