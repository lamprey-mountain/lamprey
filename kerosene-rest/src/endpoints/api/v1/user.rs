use common::{
    util::Diff,
    v1::types::{
        AuditLogEntryType, MessageSync, Permission, SERVER_ROOM_ID, UserPatch,
        UserWithRelationship,
        ack::{AckBulkItem, AckType},
        oauth::Scope,
        util::{Changes, Time},
    },
    v2::types::{MessageId, media::MediaLinkType},
};
use http::StatusCode;
use lamprey_backend_services::{
    compat::types::UserIdReq,
    services::{media::MediaLinker, messages::create2::Create},
};
use tracing::warn;

use crate::prelude::*;

#[handler(routes::user_get)]
pub async fn get(req: Req<routes::user_get::Endpoint>) -> Result<routes::user_get::Response> {
    let srv = req.services();
    let identity = req.identity();

    let mut user = match &req.inner().user_id {
        UserIdReq::UserSelf => identity.ensure_user()?.clone(),
        UserIdReq::UserId(target_user_id) => srv
            .users
            .get(*target_user_id, identity.user_id())
            .await
            .cast_internal()?,
        UserIdReq::RemoteUser(user_id, hostname) => {
            // TODO: get local user, get server info, check if user remote epoch == server sync epoch
            // NOTE: do i put epoch checks in users service, federation service, somewhere else...?
            srv.federation
                .import_user(*user_id, &hostname)
                .await
                .cast_internal()?
        }
    };

    if identity.ensure_scopes(&[Scope::Email]).is_err() {
        user.emails = None;
    }

    // TODO: move relationship fetching to users service?
    let relationship = if let Some(auth_user) = identity.user() {
        req.globals()
            .begin_read()
            .await
            .cast_internal()?
            .user_relationship_get(auth_user.id, user.id)
            .await
            .cast_internal()?
            .unwrap_or_default()
    } else {
        Default::default()
    };

    let user = UserWithRelationship {
        inner: user,
        relationship,
    };

    Ok(routes::user_get::Response { user })
}

#[handler(routes::user_update)]
pub async fn update(
    req: Req<routes::user_update::Endpoint>,
) -> Result<routes::user_update::Response> {
    let srv = req.services();
    let identity = req.identity();
    let auth_user = identity.ensure_user()?;
    auth_user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let al = req.audit_log();
    let inner = req.inner();
    let target_user_id = inner.user_id.clone().local_unwrap_or(auth_user.id)?;

    let mut perms = srv
        .perms
        .for_room3(Some(auth_user.id), SERVER_ROOM_ID)
        .await
        .cast_internal()?
        .ensure_view()?;

    if auth_user.id != target_user_id {
        perms.needs(Permission::UserManage);
    } else {
        perms.needs(Permission::UserProfileSelf);
    }
    perms.check()?;

    let mut data = req.globals().begin().await.cast_internal()?;
    let start = srv
        .users
        .get(target_user_id, Some(auth_user.id))
        .await
        .cast_internal()?;

    if !inner.patch.changes(&start) {
        return Ok(routes::user_update::Response { user: start });
    }

    let mut lm = MediaLinker::new(auth_user.id);
    let mut media_store = Vec::new();

    if let Some(maybe_avatar) = inner.patch.avatar {
        lm.delete(MediaLinkType::UserAvatar {
            user_id: target_user_id,
        });
        if let Some(avatar_media_id) = maybe_avatar {
            let media = data.media_select(avatar_media_id).await.cast_internal()?;
            if !media.metadata.is_image() {
                return Err(ApiError::with_message(
                    ErrorCode::MediaNotAnImage,
                    "avatar must be an image".to_string(),
                )
                .into());
            }
            media_store.push(media);
            lm.create(MediaLinkType::UserAvatar {
                user_id: target_user_id,
            });
        }
    }

    if let Some(maybe_banner) = inner.patch.banner {
        lm.delete(MediaLinkType::UserBanner {
            user_id: target_user_id,
        });
        if let Some(banner_media_id) = maybe_banner {
            let media = data.media_select(banner_media_id).await.cast_internal()?;
            if !media.metadata.is_image() {
                return Err(ApiError::with_message(
                    ErrorCode::MediaNotAnImage,
                    "banner must be an image".to_string(),
                )
                .into());
            }
            media_store.push(media);
            lm.create(MediaLinkType::UserBanner {
                user_id: target_user_id,
            });
        }
    }

    for m in &media_store {
        lm.media(m);
    }

    data.user_update(target_user_id, inner.patch.clone())
        .await
        .cast_internal()?;
    lm.write(&mut *data).await?;

    data.commit().await.cast_internal()?;
    srv.users.invalidate(target_user_id).await;

    let user = srv
        .users
        .get(target_user_id, Some(auth_user.id))
        .await
        .cast_internal()?;

    let changes = Changes::new()
        .change("name", &start.name, &user.name)
        .change("description", &start.description, &user.description)
        .change("avatar", &start.avatar, &user.avatar)
        .change("banner", &start.banner, &user.banner)
        .build();

    al.push(
        target_user_id.into_inner().into(),
        AuditLogEntryType::UserUpdate {
            changes: changes.clone(),
        },
        req.headers().reason.clone(),
    )
    .await
    .success()
    .await;

    if auth_user.id != target_user_id {
        al.push(
            SERVER_ROOM_ID.into_inner().into(),
            AuditLogEntryType::UserUpdate { changes },
            req.headers().reason.clone(),
        )
        .await
        .success()
        .await;
    }

    Ok(routes::user_update::Response { user })
}

export_routes!(get, update);
