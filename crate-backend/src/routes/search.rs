use std::sync::Arc;

use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::Json;
use common::v1::routes;
use common::v1::types::search::{
    ChannelSearchRequest, MessageSearch, MessageSearchRequest, RoomSearchRequest,
};
use common::v1::types::{Channel, ChannelId, PaginationQuery, PaginationResponse, Room, RoomId};
use lamprey_macros::handler;
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::{routes2, Error, ServerState};

use super::util::Auth;
use crate::error::Result;

/// Search messages
#[handler(routes::search_messages)]
pub async fn search_messages(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::search_messages::Request,
) -> Result<impl IntoResponse> {
    req.search.validate()?;
    let res = s
        .services()
        .search
        .search_messages(auth.user.id, req.search)
        .await?;
    Ok(Json(res))
}

/// Search channels
#[handler(routes::search_channels)]
pub async fn search_channels(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::search_channels::Request,
) -> Result<impl IntoResponse> {
    req.search.validate()?;
    let res = s
        .services()
        .search
        .search_channels(auth.user.id, req.search, req.pagination)
        .await?;
    Ok(Json(res))
}

/// Search rooms (TODO)
#[handler(routes::search_rooms)]
pub async fn search_rooms(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::search_rooms::Request,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(search_messages))
        .routes(routes2!(search_channels))
        .routes(routes2!(search_rooms))
}
