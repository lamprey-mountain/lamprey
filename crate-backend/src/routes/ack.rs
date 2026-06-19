use std::collections::HashSet;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use common::v1::routes;
use common::v1::types::MessageSync;
use common::v1::types::ack::AckState;
use common::v1::types::application::Scope;
use lamprey_macros::handler;
use tracing::warn;
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

    let mut channel_ids = HashSet::new();
    let mut unknown_auth = false;

    for ack in &req.body.acks {
        if let Some(channel_id) = ack.ty.channel_id() {
            channel_ids.insert(channel_id);
        } else {
            unknown_auth = true;
        }
    }

    if unknown_auth {
        warn!("unknown auth check for this ack type, allowing");
    }

    for &channel_id in &channel_ids {
        srv.perms
            .for_channel3(Some(auth.user.id), channel_id)
            .await?
            .ensure_view()?
            .check()?;
    }

    if !req.body.acks.is_empty() {
        data.unread_ack_bulk(auth.user.id, &req.body.acks).await?;

        for &channel_id in &channel_ids {
            srv.channels.invalidate_user(channel_id, auth.user.id).await;
        }

        s.broadcast(MessageSync::PassiveAck {
            user_id: auth.user.id,
            ack_states: req
                .body
                .acks
                .into_iter()
                .map(|a| AckState {
                    ty: a.ty,
                    unread: false,
                })
                .collect(),
        })?;
    }

    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new().routes(routes2!(ack_bulk))
}
