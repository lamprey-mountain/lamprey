use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
};
use common::v1::types::WebhookId;

use crate::{error::Error, Result, ServerState};

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
pub async fn webhook_execute_slack(
    Path((_webhook_id, _token)): Path<(WebhookId, String)>,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}
