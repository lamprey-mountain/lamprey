use std::sync::Arc;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::ack::AckBulk;
use common::v1::types::{MessageSync, Permission};
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::ServerState;

use super::util::Auth;
use crate::error::Result;

/// Ack bulk
#[utoipa::path(
    post,
    path = "/ack",
    tags = ["ack"],
    responses(
        (status = NO_CONTENT, description = "ok"),
    )
)]
async fn ack_bulk(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<AckBulk>,
) -> Result<impl IntoResponse> {
    json.validate()?;

    let data = s.data();
    let srv = s.services();

    let mut valid_acks = Vec::new();

    for ack in json.acks {
        let perms = srv.perms.for_channel(auth.user.id, ack.channel_id).await?;
        if !perms.has(Permission::ViewChannel) {
            continue;
        }

        if ack.message_id.is_none() {
            continue;
        }

        valid_acks.push(ack);
    }

    if !valid_acks.is_empty() {
        data.unread_ack_bulk(auth.user.id, valid_acks.clone())
            .await?;

        for ack in valid_acks {
            srv.channels
                .invalidate_user(ack.channel_id, auth.user.id)
                .await;
            s.broadcast(MessageSync::ChannelAck {
                user_id: auth.user.id,
                channel_id: ack.channel_id,
                message_id: ack.message_id.unwrap(),
                version_id: ack.version_id,
            })?;
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new().routes(routes!(ack_bulk))
}
