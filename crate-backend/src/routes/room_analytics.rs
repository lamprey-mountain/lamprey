use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use common::v1::types::{
    room_analytics::{
        RoomAnalyticsChannel, RoomAnalyticsChannelParams, RoomAnalyticsInvites,
        RoomAnalyticsMembersCount, RoomAnalyticsMembersJoin, RoomAnalyticsMembersLeave,
        RoomAnalyticsOverview, RoomAnalyticsParams,
    },
    Permission, RoomId,
};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    error::{Error, Result},
    ServerState,
};

use super::util::Auth2;

/// Room analytics members count
#[utoipa::path(
    get,
    path = "/room/{room_id}/analytics/members-count",
    tags = ["room_analytics"],
    responses((status = 200, description = "success", body = Vec<RoomAnalyticsMembersCount>)),
)]
async fn room_analytics_members_count(
    auth: Auth2,
    Path(room_id): Path<RoomId>,
    Query(q): Query<RoomAnalyticsParams>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let srv = s.services();
    let data = s.data();

    let perms = srv.perms.for_room(auth.user.id, room_id).await?;
    perms.ensure(Permission::ViewAnalytics)?;

    let datapoints = data.room_analytics_members_count(room_id, q).await?;
    Ok(Json(datapoints))
}

/// Room analytics members joined
#[utoipa::path(
    get,
    path = "/room/{room_id}/analytics/members-join",
    tags = ["room_analytics"],
    responses((status = 200, description = "success", body = Vec<RoomAnalyticsMembersJoin>)),
)]
async fn room_analytics_members_join(
    auth: Auth2,
    Path(room_id): Path<RoomId>,
    Query(q): Query<RoomAnalyticsParams>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let perms = s.services().perms.for_room(auth.user.id, room_id).await?;
    perms.ensure(Permission::ViewAnalytics)?;
    let res = s.data().room_analytics_members_join(room_id, q).await?;
    Ok(Json(res))
}

/// Room analytics members left
#[utoipa::path(
    get,
    path = "/room/{room_id}/analytics/members-leave",
    tags = ["room_analytics"],
    responses((status = 200, description = "success", body = Vec<RoomAnalyticsMembersLeave>)),
)]
async fn room_analytics_members_leave(
    auth: Auth2,
    Path(room_id): Path<RoomId>,
    Query(q): Query<RoomAnalyticsParams>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let perms = s.services().perms.for_room(auth.user.id, room_id).await?;
    perms.ensure(Permission::ViewAnalytics)?;
    let res = s.data().room_analytics_members_leave(room_id, q).await?;
    Ok(Json(res))
}

/// Room analytics channels
#[utoipa::path(
    get,
    path = "/room/{room_id}/analytics/channels",
    tags = ["room_analytics"],
    responses((status = 200, description = "success", body = Vec<RoomAnalyticsChannel>)),
)]
async fn room_analytics_channels(
    auth: Auth2,
    Path(room_id): Path<RoomId>,
    Query(q): Query<RoomAnalyticsParams>,
    Query(q2): Query<RoomAnalyticsChannelParams>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let perms = s.services().perms.for_room(auth.user.id, room_id).await?;
    perms.ensure(Permission::ViewAnalytics)?;
    let res = s.data().room_analytics_channels(room_id, q, q2).await?;
    Ok(Json(res))
}

/// Room analytics overview
///
/// aggregate all stats from all channels
#[utoipa::path(
    get,
    path = "/room/{room_id}/analytics/overview",
    tags = ["room_analytics"],
    responses((status = 200, description = "success", body = Vec<RoomAnalyticsOverview>)),
)]
async fn room_analytics_overview(
    auth: Auth2,
    Path(room_id): Path<RoomId>,
    Query(q): Query<RoomAnalyticsParams>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let perms = s.services().perms.for_room(auth.user.id, room_id).await?;
    perms.ensure(Permission::ViewAnalytics)?;
    let res = s.data().room_analytics_overview(room_id, q).await?;
    Ok(Json(res))
}

/// Room analytics invites (TODO)
#[utoipa::path(
    get,
    path = "/room/{room_id}/analytics/invites",
    tags = ["room_analytics"],
    responses((status = 200, description = "success", body = Vec<RoomAnalyticsInvites>)),
)]
async fn room_analytics_invites(
    auth: Auth2,
    Path(_room_id): Path<RoomId>,
    Query(_q): Query<RoomAnalyticsParams>,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

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
