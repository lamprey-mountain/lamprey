use common::v1::types::{PaginationResponse, oauth::Scope};
use lamprey_backend_services::services::search::SearchRoomsVisibility;

use crate::prelude::*;

#[handler(routes::search_messages)]
pub async fn messages(
    req: Req<routes::search_messages::Endpoint>,
) -> Result<routes::search_messages::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let body = req.inner();
    body.search
        .validate()
        .map_err(|e| ApiError::with_message(ErrorCode::InvalidData, e.to_string()))?;

    let search = req
        .services()
        .search
        .search_messages(user.id, body.search.clone())
        .await
        .cast_internal()?;

    Ok(routes::search_messages::Response { search })
}

#[handler(routes::search_channels)]
pub async fn channels(
    req: Req<routes::search_channels::Endpoint>,
) -> Result<routes::search_channels::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let body = req.inner();
    body.search
        .validate()
        .map_err(|e| ApiError::with_message(ErrorCode::InvalidData, e.to_string()))?;

    let channels = req
        .services()
        .search
        .search_channels(user.id, body.search.clone())
        .await
        .cast_internal()?;

    Ok(routes::search_channels::Response { channels })
}

#[handler(routes::search_rooms)]
pub async fn rooms(
    req: Req<routes::search_rooms::Endpoint>,
) -> Result<routes::search_rooms::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Full])?;

    let body = req.inner();
    body.search
        .validate()
        .map_err(|e| ApiError::with_message(ErrorCode::InvalidData, e.to_string()))?;

    let search = req
        .services()
        .search
        .search_rooms(SearchRoomsVisibility::PublicOnly, body.search.clone())
        .await
        .cast_internal()?;

    Ok(routes::search_rooms::Response {
        rooms: PaginationResponse {
            items: search.rooms,
            total: search.total,
            has_more: search.has_more,
            cursor: search.cursor,
        },
    })
}

export_routes!(messages, channels, rooms);
