use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::routes;
use common::v1::types::application::Scope;
use common::v1::types::room_analytics::{
    RoomAnalyticsChannel, RoomAnalyticsChannelParams, RoomAnalyticsInvites,
    RoomAnalyticsMembersCount, RoomAnalyticsMembersJoin, RoomAnalyticsMembersLeave,
    RoomAnalyticsOverview, RoomAnalyticsParams,
};
use common::v1::types::{Permission, RoomId};
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;

use crate::error::{Error, Result};
use crate::routes::util::Auth;
use crate::{routes2, ServerState};

/// Room analytics members count
#[handler(routes::room_analytics_members_count)]
async fn room_analytics_members_count(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_analytics_members_count::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    let srv = s.services();
    let data = s.data();

    let perms = srv.perms.for_room(auth.user.id, req.room_id).await?;
    perms.ensure(Permission::AnalyticsView)?;

    let datapoints = data
        .room_analytics_members_count(req.room_id, req.params)
        .await?;
    Ok(Json(datapoints))
}

/// Room analytics members joined
#[handler(routes::room_analytics_members_join)]
async fn room_analytics_members_join(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_analytics_members_join::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    let perms = s
        .services()
        .perms
        .for_room(auth.user.id, req.room_id)
        .await?;
    perms.ensure(Permission::AnalyticsView)?;
    let res = s
        .data()
        .room_analytics_members_join(req.room_id, req.params)
        .await?;
    Ok(Json(res))
}

/// Room analytics members left
#[handler(routes::room_analytics_members_leave)]
async fn room_analytics_members_leave(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_analytics_members_leave::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    let perms = s
        .services()
        .perms
        .for_room(auth.user.id, req.room_id)
        .await?;
    perms.ensure(Permission::AnalyticsView)?;
    let res = s
        .data()
        .room_analytics_members_leave(req.room_id, req.params)
        .await?;
    Ok(Json(res))
}

/// Room analytics channels
#[handler(routes::room_analytics_channels)]
async fn room_analytics_channels(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_analytics_channels::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    let perms = s
        .services()
        .perms
        .for_room(auth.user.id, req.room_id)
        .await?;
    perms.ensure(Permission::AnalyticsView)?;
    let res = s
        .data()
        .room_analytics_channels(req.room_id, req.params, req.channel_params)
        .await?;
    Ok(Json(res))
}

/// Room analytics overview
///
/// aggregate all stats from all channels
#[handler(routes::room_analytics_overview)]
async fn room_analytics_overview(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_analytics_overview::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    let perms = s
        .services()
        .perms
        .for_room(auth.user.id, req.room_id)
        .await?;
    perms.ensure(Permission::AnalyticsView)?;
    let res = s
        .data()
        .room_analytics_overview(req.room_id, req.params)
        .await?;
    Ok(Json(res))
}

/// Room analytics invites (TODO)
#[handler(routes::room_analytics_invites)]
async fn room_analytics_invites(
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::room_analytics_invites::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    Ok(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(room_analytics_members_count))
        .routes(routes2!(room_analytics_members_join))
        .routes(routes2!(room_analytics_members_leave))
        .routes(routes2!(room_analytics_channels))
        .routes(routes2!(room_analytics_overview))
        .routes(routes2!(room_analytics_invites))
}
