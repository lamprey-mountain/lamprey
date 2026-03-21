use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use common::v1::routes;
use common::v1::types::application::Scope;
use common::v1::types::{MessageSync, Permission};
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;

use crate::{routes2, ServerState};

use super::util::Auth;
use crate::error::Result;

/// Ack bulk
#[handler(routes::ack_bulk)]
async fn ack_bulk(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::ack_bulk::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;

    let data = s.data();
    let srv = s.services();

    let mut valid_acks = Vec::new();

    for ack in req.ack.acks {
        let perms = srv.perms.for_channel(auth.user.id, ack.channel_id).await?;
        if !perms.has(Permission::ChannelView) {
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
    OpenApiRouter::new().routes(routes2!(ack_bulk))
}
