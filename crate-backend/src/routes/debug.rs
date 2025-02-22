use std::sync::Arc;

use axum::response::IntoResponse;
use axum::{extract::State, Json};
use types::{UrlEmbed, UrlEmbedRequest};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::ServerState;

use super::util::Auth;
use crate::error::Result;

/// Embed a url
#[utoipa::path(
    post,
    path = "/debug/embed-url",
    tags = ["debug"],
    responses(
        (status = OK, body = UrlEmbed, description = "success"),
    )
)]
pub async fn debug_embed_url(
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<UrlEmbedRequest>,
) -> Result<impl IntoResponse> {
    let embed = s.services().url_embed.generate(user_id, json.url).await?;
    Ok(Json(embed))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new().routes(routes!(debug_embed_url))
}
