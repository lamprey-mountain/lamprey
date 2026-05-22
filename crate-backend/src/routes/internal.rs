use std::sync::Arc;

use axum::{
    extract::{ws::WebSocketUpgrade, State},
    response::IntoResponse,
};
use common::v1::types::SfuId;
use http::HeaderMap;
use tracing::error;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::error::Result;
use crate::{Error, ServerState};

/// Internal rpc
#[utoipa::path(
    get,
    path = "/internal/rpc",
    tags = ["internal"],
    responses((status = 101, description = "Switching Protocols")),
)]
async fn internal_rpc(
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let Some(v) = &s.config.voice else {
        return Err(Error::Unimplemented);
    };

    let auth = headers
        .get("authorization")
        .ok_or(Error::MissingAuth)?
        .to_str()?;

    if auth != format!("Server {}", v.token) {
        return Err(Error::MissingAuth);
    }

    Ok(ws.on_upgrade(move |socket| async move {
        let srv = s.services();
        if let Err(e) = srv.voice.sfu_handle_connect(SfuId::new(), socket).await {
            error!("Failed to connect to SFU: {:?}", e);
            // NOTE: do i need to destroy the sfu here?
            return;
        }
    }))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new().routes(routes!(internal_rpc))
}
