use common::v1::types::{Permission, oauth::Scope};

use crate::prelude::*;

#[handler(routes::room_analytics_members_count)]
pub async fn members_count(
    req: Req<routes::room_analytics_members_count::Endpoint>,
) -> Result<routes::room_analytics_members_count::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    let mut perms = srv
        .perms
        .for_room3(Some(user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    perms.needs(Permission::AnalyticsView);
    perms.check()?;

    let mut data = req.globals().begin_read().await.cast_internal()?;
    let analytics = data
        .room_analytics_members_count(body.room_id, body.params.clone())
        .await
        .cast_internal()?;

    Ok(routes::room_analytics_members_count::Response { analytics })
}

#[handler(routes::room_analytics_members_join)]
pub async fn members_join(
    req: Req<routes::room_analytics_members_join::Endpoint>,
) -> Result<routes::room_analytics_members_join::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    let mut perms = srv
        .perms
        .for_room3(Some(user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    perms.needs(Permission::AnalyticsView);
    perms.check()?;

    let mut data = req.globals().begin_read().await.cast_internal()?;
    let analytics = data
        .room_analytics_members_join(body.room_id, body.params.clone())
        .await
        .cast_internal()?;

    Ok(routes::room_analytics_members_join::Response { analytics })
}

#[handler(routes::room_analytics_members_leave)]
pub async fn members_leave(
    req: Req<routes::room_analytics_members_leave::Endpoint>,
) -> Result<routes::room_analytics_members_leave::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    let mut perms = srv
        .perms
        .for_room3(Some(user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    perms.needs(Permission::AnalyticsView);
    perms.check()?;

    let mut data = req.globals().begin_read().await.cast_internal()?;
    let analytics = data
        .room_analytics_members_leave(body.room_id, body.params.clone())
        .await
        .cast_internal()?;

    Ok(routes::room_analytics_members_leave::Response { analytics })
}

#[handler(routes::room_analytics_channels)]
pub async fn channels(
    req: Req<routes::room_analytics_channels::Endpoint>,
) -> Result<routes::room_analytics_channels::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    let mut perms = srv
        .perms
        .for_room3(Some(user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    perms.needs(Permission::AnalyticsView);
    perms.check()?;

    let mut data = req.globals().begin_read().await.cast_internal()?;
    let analytics = data
        .room_analytics_channels(
            body.room_id,
            body.params.clone(),
            body.channel_params.clone(),
        )
        .await
        .cast_internal()?;

    Ok(routes::room_analytics_channels::Response { analytics })
}

#[handler(routes::room_analytics_overview)]
pub async fn overview(
    req: Req<routes::room_analytics_overview::Endpoint>,
) -> Result<routes::room_analytics_overview::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    let mut perms = srv
        .perms
        .for_room3(Some(user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    perms.needs(Permission::AnalyticsView);
    perms.check()?;

    let mut data = req.globals().begin_read().await.cast_internal()?;
    let analytics = data
        .room_analytics_overview(body.room_id, body.params.clone())
        .await
        .cast_internal()?;

    Ok(routes::room_analytics_overview::Response { analytics })
}

#[handler(routes::room_analytics_invites)]
pub async fn invites(
    req: Req<routes::room_analytics_invites::Endpoint>,
) -> Result<routes::room_analytics_invites::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    srv.perms
        .for_room3(Some(user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .needs(Permission::AnalyticsView)
        .check()?;

    Err(Error::Unimplemented)
}

export_routes!(
    members_count,
    members_join,
    members_leave,
    channels,
    overview,
    invites,
);
