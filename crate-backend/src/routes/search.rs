use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::Query;
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::search::{SearchChannelsRequest, SearchMessageRequest, SearchRoomsRequest};
use common::v1::types::{
    Channel, ChannelId, Message, MessageId, PaginationQuery, PaginationResponse, Room, RoomId,
};
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::{Error, ServerState};

use super::util::Auth;
use crate::error::Result;

/// Search messages
#[utoipa::path(
    post,
    path = "/search/message",
    tags = ["search"],
    responses(
        (status = OK, body = PaginationResponse<Message>, description = "success"),
    )
)]
pub async fn search_messages(
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    Query(q): Query<PaginationQuery<MessageId>>,
    Json(json): Json<SearchMessageRequest>,
) -> Result<impl IntoResponse> {
    json.validate()?;
    let data = s.data();
    let srv = s.services();
    let vis = srv.channels.list_user_room_channels(auth_user.id).await?;
    let mut res = data.search_message(auth_user.id, json, q, &vis).await?;

    // group messages by channel id
    let mut channel_message_indices: HashMap<ChannelId, Vec<usize>> = HashMap::new();
    for (i, message) in res.items.iter().enumerate() {
        channel_message_indices
            .entry(message.channel_id)
            .or_default()
            .push(i);
    }

    // TODO: avoid cloning
    // populate reactions
    for (channel_id, indices) in channel_message_indices {
        let mut temp_messages: Vec<Message> =
            indices.iter().map(|&i| res.items[i].clone()).collect();

        srv.messages
            .populate_reactions(channel_id, auth_user.id, &mut temp_messages)
            .await?;

        for (i, original_index) in indices.iter().enumerate() {
            res.items[*original_index].reactions = temp_messages[i].reactions.clone();
        }
    }

    for message in &mut res.items {
        s.presign_message(message).await?;
    }
    Ok(Json(res))
}

/// Search channels
#[utoipa::path(
    post,
    path = "/search/channels",
    tags = ["search"],
    responses(
        (status = OK, body = PaginationResponse<Channel>, description = "success"),
    )
)]
pub async fn search_channels(
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    Query(q): Query<PaginationQuery<ChannelId>>,
    Json(json): Json<SearchChannelsRequest>,
) -> Result<impl IntoResponse> {
    json.validate()?;
    let data = s.data();
    let srv = s.services();
    let vis = srv.channels.list_user_room_channels(auth_user.id).await?;
    let res = data.search_channel(auth_user.id, json, q, &vis).await?;
    Ok(Json(res))
}

/// Search rooms (TODO)
#[utoipa::path(
    post,
    path = "/search/room",
    tags = ["search"],
    responses(
        (status = OK, body = PaginationResponse<Room>, description = "success"),
    )
)]
pub async fn search_rooms(
    Auth(_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    Query(_q): Query<PaginationQuery<RoomId>>,
    Json(_json): Json<SearchRoomsRequest>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(search_messages))
        .routes(routes!(search_channels))
        .routes(routes!(search_rooms))
}
