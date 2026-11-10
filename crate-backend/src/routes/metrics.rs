use axum::{extract::State, response::IntoResponse};
use common::v1::types::application::Scope;
use common::v1::types::error::{ApiError, ErrorCode};
use prometheus::{Encoder, TextEncoder};

use crate::prelude::*;
use crate::{routes::util::Auth, types::Permission, types::SERVER_ROOM_ID};

pub async fn get_metrics(auth: Auth, State(s): State<Globals>) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let mut perms = s
        .services()
        .perms
        .for_room3(Some(auth.user.id), SERVER_ROOM_ID)
        .await?
        .ensure_view()?;
    perms.needs(Permission::ServerMetrics);
    perms.check()?;

    let registry = kerosene_core::observability::prometheus_registry()
        .ok_or_else(|| ApiError::from_code(ErrorCode::FailedToEncodeMetrics))?;

    let encoder = TextEncoder::new();
    let mut buffer = vec![];
    encoder
        .encode(&registry.gather(), &mut buffer)
        .map_err(|_| ApiError::from_code(ErrorCode::FailedToEncodeMetrics))?;

    Ok(buffer)
}
