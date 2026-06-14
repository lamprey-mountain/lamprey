use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use common::v1::{routes, types::ack::AckType};
use common::v1::types::MessageSync;
use common::v1::types::application::Scope;
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;

use crate::{ServerState, routes2};

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

    let mut data = s.data();
    let srv = s.services();

    let mut valid_acks = Vec::new();

    // PERF: somehow check in bulk or parallel?
    // TODO: handle other `AckType`s
    for ack in req.body.acks {
        match &ack.ty {
            AckType::Message { channel_id, message_id, .. } => {
                srv.perms
                    .for_channel3(Some(auth.user.id), *channel_id)
                    .await?
                    .ensure_view()?
                    .check()?;
                valid_acks.push(ack.clone());
            }
            _ => continue,
        }
    }

    if !valid_acks.is_empty() {
        data.unread_ack_bulk(auth.user.id, &valid_acks)
            .await?;

        for ack in valid_acks {
            if let AckType::Message { channel_id, message_id, .. } = ack.ty {
                srv.channels
                    .invalidate_user(channel_id, auth.user.id)
                    .await;
                s.broadcast(MessageSync::ChannelAck {
                    user_id: auth.user.id,
                    channel_id,
                    message_id,
                    version_id: uuid::Uuid::nil().into(),
                })?;
            }
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new().routes(routes2!(ack_bulk))
}
