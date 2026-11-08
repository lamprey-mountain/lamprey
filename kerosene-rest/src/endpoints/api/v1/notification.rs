use common::v1::types::NotificationId;
use common::v1::types::notifications::{Notification, NotificationType};
use common::v1::types::oauth::Scope;
use common::v1::types::util::Time;

use crate::prelude::*;

#[handler(routes::inbox_get)]
pub async fn inbox_get(
    req: Req<routes::inbox_get::Endpoint>,
) -> Result<routes::inbox_get::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let body = req.inner();
    let inbox = req
        .services()
        .notifications
        .inbox_query(user.id, body.pagination.clone(), body.params.clone())
        .await
        .cast_internal()?;

    Ok(routes::inbox_get::Response { inbox })
}

#[handler(routes::inbox_post)]
pub async fn inbox_post(
    req: Req<routes::inbox_post::Endpoint>,
) -> Result<routes::inbox_post::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();

    srv.perms
        .for_channel3(Some(user.id), body.notification.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .check()?;

    let room_id = srv
        .channels
        .get(body.notification.channel_id, Some(user.id))
        .await
        .ok()
        .and_then(|ch| ch.room_id);

    let mut data = globals.begin().await.cast_internal()?;
    let message = data
        .message_get(body.notification.channel_id, body.notification.message_id)
        .await
        .cast_internal()?;

    let mention_role = match room_id {
        Some(room_id) if !message.latest_version.mentions.roles.is_empty() => data
            .room_member_get(room_id, user.id)
            .await
            .ok()
            .is_some_and(|member| {
                message
                    .latest_version
                    .mentions
                    .roles
                    .iter()
                    .any(|role| member.roles.contains(&role.id))
            }),
        _ => false,
    };

    let notification = Notification {
        id: NotificationId::new(),
        ty: NotificationType::Message {
            room_id,
            channel_id: body.notification.channel_id,
            message_id: body.notification.message_id,
            user_id: message.author_id,
            mention_user: message
                .latest_version
                .mentions
                .users
                .iter()
                .any(|u| u.id == user.id),
            mention_everyone: message.latest_version.mentions.everyone,
            mention_role,
            reply: message.reply_id().is_some(),
        },
        added_at: body.notification.added_at.unwrap_or_else(Time::now_utc),
        read_at: None,
        note: None,
    };

    data.notification_add(user.id, notification.clone())
        .await
        .cast_internal()?;
    data.commit().await.cast_internal()?;

    Ok(routes::inbox_post::Response { notification })
}

#[handler(routes::inbox_mark_read)]
pub async fn inbox_mark_read(
    req: Req<routes::inbox_mark_read::Endpoint>,
) -> Result<routes::inbox_mark_read::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    for channel_id in &body.mark_read.channel_ids {
        srv.perms
            .for_channel3(Some(user.id), *channel_id)
            .await
            .cast_internal()?
            .ensure_view()?
            .check()?;
    }

    let mut data = req.globals().begin().await.cast_internal()?;
    data.notification_mark_read(user.id, body.mark_read.clone())
        .await
        .cast_internal()?;
    data.commit().await.cast_internal()?;

    Ok(routes::inbox_mark_read::Response {})
}

#[handler(routes::inbox_mark_unread)]
pub async fn inbox_mark_unread(
    req: Req<routes::inbox_mark_unread::Endpoint>,
) -> Result<routes::inbox_mark_unread::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    for channel_id in &body.mark_unread.channel_ids {
        srv.perms
            .for_channel3(Some(user.id), *channel_id)
            .await
            .cast_internal()?
            .ensure_view()?
            .check()?;
    }

    let mut data = req.globals().begin().await.cast_internal()?;
    data.notification_mark_unread(user.id, body.mark_unread.clone())
        .await
        .cast_internal()?;
    data.commit().await.cast_internal()?;

    Ok(routes::inbox_mark_unread::Response {})
}

#[handler(routes::inbox_flush)]
pub async fn inbox_flush(
    req: Req<routes::inbox_flush::Endpoint>,
) -> Result<routes::inbox_flush::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    if let Some(channel_ids) = &body.flush.channel_ids {
        for channel_id in channel_ids {
            srv.perms
                .for_channel3(Some(user.id), *channel_id)
                .await
                .cast_internal()?
                .ensure_view()?
                .check()?;
        }
    }

    let mut data = req.globals().begin().await.cast_internal()?;
    data.notification_flush(user.id, body.flush.clone())
        .await
        .cast_internal()?;
    data.commit().await.cast_internal()?;

    Ok(routes::inbox_flush::Response {})
}

export_routes!(
    inbox_get,
    inbox_post,
    inbox_mark_read,
    inbox_mark_unread,
    inbox_flush
);
