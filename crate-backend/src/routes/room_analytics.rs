use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
};
use common::v1::types::{
    room_analytics::{
        RoomAnalyticsChannel, RoomAnalyticsChannelParams, RoomAnalyticsInvites,
        RoomAnalyticsMembersCount, RoomAnalyticsMembersJoin, RoomAnalyticsMembersLeave,
        RoomAnalyticsOverview, RoomAnalyticsParams,
    },
    RoomId,
};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    error::{Error, Result},
    ServerState,
};

use super::util::Auth;

/// Room analytics members count (TODO)
#[utoipa::path(
    get,
    path = "/room/{room_id}/analytics/members-count",
    tags = ["room_analytics"],
    responses((status = 200, description = "success", body = Vec<RoomAnalyticsMembersCount>)),
)]
async fn room_analytics_members_count(
    Auth(auth_user): Auth,
    Path(_room_id): Path<RoomId>,
    Query(_q): Query<RoomAnalyticsParams>,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;

    Ok(Error::Unimplemented)
}

/// Room analytics members joined (TODO)
#[utoipa::path(
    get,
    path = "/room/{room_id}/analytics/members-join",
    tags = ["room_analytics"],
    responses((status = 200, description = "success", body = Vec<RoomAnalyticsMembersJoin>)),
)]
async fn room_analytics_members_join(
    Auth(auth_user): Auth,
    Path(_room_id): Path<RoomId>,
    Query(_q): Query<RoomAnalyticsParams>,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;

    Ok(Error::Unimplemented)
}

/// Room analytics members left (TODO)
#[utoipa::path(
    get,
    path = "/room/{room_id}/analytics/members-leave",
    tags = ["room_analytics"],
    responses((status = 200, description = "success", body = Vec<RoomAnalyticsMembersLeave>)),
)]
async fn room_analytics_members_leave(
    Auth(auth_user): Auth,
    Path(_room_id): Path<RoomId>,
    Query(_q): Query<RoomAnalyticsParams>,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;

    Ok(Error::Unimplemented)
}

/// Room analytics channels (TODO)
#[utoipa::path(
    get,
    path = "/room/{room_id}/analytics/channels",
    tags = ["room_analytics"],
    responses((status = 200, description = "success", body = Vec<RoomAnalyticsChannel>)),
)]
async fn room_analytics_channels(
    Auth(auth_user): Auth,
    Path(_room_id): Path<RoomId>,
    Query(_q): Query<RoomAnalyticsParams>,
    Query(_q2): Query<RoomAnalyticsChannelParams>,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;

    Ok(Error::Unimplemented)
}

/// Room analytics overview (TODO)
///
/// aggregate all stats from all channels
#[utoipa::path(
    get,
    path = "/room/{room_id}/analytics/overview",
    tags = ["room_analytics"],
    responses((status = 200, description = "success", body = Vec<RoomAnalyticsOverview>)),
)]
async fn room_analytics_overview(
    Auth(auth_user): Auth,
    Path(_room_id): Path<RoomId>,
    Query(_q): Query<RoomAnalyticsParams>,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;

    Ok(Error::Unimplemented)
}

/// Room analytics invites (TODO)
#[utoipa::path(
    get,
    path = "/room/{room_id}/analytics/invites",
    tags = ["room_analytics"],
    responses((status = 200, description = "success", body = Vec<RoomAnalyticsInvites>)),
)]
async fn room_analytics_invites(
    Auth(auth_user): Auth,
    Path(_room_id): Path<RoomId>,
    Query(_q): Query<RoomAnalyticsParams>,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;

    Ok(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(room_analytics_members_count))
        .routes(routes!(room_analytics_members_join))
        .routes(routes!(room_analytics_members_leave))
        .routes(routes!(room_analytics_channels))
        .routes(routes!(room_analytics_overview))
        .routes(routes!(room_analytics_invites))
}
