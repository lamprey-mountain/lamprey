use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::routes;
use common::v1::types::application::Scope;
use common::v1::types::util::Time;
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;

use crate::routes::util::auth::Auth4;
use crate::routes2;
use crate::{error::Result, ServerState};

/// Flume create
#[handler(routes::flume::flume_create)]
async fn flume_create(
    auth: Auth4,
    State(s): State<Arc<ServerState>>,
    req: routes::flume::flume_create::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_user()?.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;

    let header_timestamp = req.timestamp.and_then(|secs| {
        time::OffsetDateTime::from_unix_timestamp(secs)
            .ok()
            .map(Time::from)
    });

    let (status, message) = s
        .services()
        .messages
        .flume_create(
            req.channel_id,
            &auth,
            req.idempotency_key,
            req.flume,
            header_timestamp,
        )
        .await?;

    Ok((status, Json(message)))
}

/// Flume ping
#[handler(routes::flume::flume_ping)]
async fn flume_ping(
    auth: Auth4,
    State(s): State<Arc<ServerState>>,
    req: routes::flume::flume_ping::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_user()?.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;

    s.services()
        .messages
        .flume_ping(req.channel_id, req.message_id, &auth)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Flume commit
#[handler(routes::flume::flume_commit)]
async fn flume_commit(
    auth: Auth4,
    State(s): State<Arc<ServerState>>,
    req: routes::flume::flume_commit::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_user()?.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;

    let message = s
        .services()
        .messages
        .flume_commit(req.channel_id, req.message_id)
        .await?;

    Ok((StatusCode::OK, Json(message)))
}

/// Flume delta
#[handler(routes::flume::flume_delta)]
async fn flume_delta(
    auth: Auth4,
    State(s): State<Arc<ServerState>>,
    req: routes::flume::flume_delta::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_user()?.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;

    let status = s
        .services()
        .messages
        .flume_update(req.channel_id, req.message_id, &auth, req.delta)
        .await?;

    Ok(status)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(flume_create))
        .routes(routes2!(flume_ping))
        .routes(routes2!(flume_commit))
        .routes(routes2!(flume_delta))
}
