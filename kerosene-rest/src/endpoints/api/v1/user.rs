use common::{
    util::Diff,
    v1::types::{
        AuditLogEntryType, MessageSync, Permission, SERVER_ROOM_ID, SessionStatus, Suspended,
        UserWithRelationship,
        oauth::Scope,
        util::{Changes, Time},
    },
    v2::types::media::MediaLinkType,
};
use lamprey_backend_services::{
    compat::types::{DbUserCreate, RoomMemberPut, UserIdReq},
    services::media::MediaLinker,
};

use crate::prelude::*;

#[handler(routes::user_get)]
pub async fn get(req: Req<routes::user_get::Endpoint>) -> Result<routes::user_get::Response> {
    let srv = req.services();
    let identity = req.identity();

    let mut user = match &req.inner().user_id {
        UserIdReq::UserSelf => {
            let auth_user = identity.ensure_user()?;
            srv.users
                .get(auth_user.id, identity.user_id())
                .await
                .cast_internal()?
        }
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

    let mut avatar_media = None;
    if let Some(Some(avatar_media_id)) = inner.patch.avatar {
        let media = data.media_select(avatar_media_id).await.cast_internal()?;
        if !media.metadata.is_image() {
            return Err(ApiError::with_message(
                ErrorCode::MediaNotAnImage,
                "avatar must be an image".to_string(),
            )
            .into());
        }
        avatar_media = Some(media);
    }

    let mut banner_media = None;
    if let Some(Some(banner_media_id)) = inner.patch.banner {
        let media = data.media_select(banner_media_id).await.cast_internal()?;
        if !media.metadata.is_image() {
            return Err(ApiError::with_message(
                ErrorCode::MediaNotAnImage,
                "banner must be an image".to_string(),
            )
            .into());
        }
        banner_media = Some(media);
    }

    data.user_update(target_user_id, inner.patch.clone())
        .await
        .cast_internal()?;

    if inner.patch.avatar.is_some() {
        let mut lm = MediaLinker::new(auth_user.id);
        lm.delete(MediaLinkType::UserAvatar {
            user_id: target_user_id,
        });
        if let Some(media) = &avatar_media {
            lm.create(MediaLinkType::UserAvatar {
                user_id: target_user_id,
            })
            .media(media);
        }
        lm.write(&mut *data).await?;
    }

    if inner.patch.banner.is_some() {
        let mut lm = MediaLinker::new(auth_user.id);
        lm.delete(MediaLinkType::UserBanner {
            user_id: target_user_id,
        });
        if let Some(media) = &banner_media {
            lm.create(MediaLinkType::UserBanner {
                user_id: target_user_id,
            })
            .media(media);
        }
        lm.write(&mut *data).await?;
    }

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
    )
    .success();

    if auth_user.id != target_user_id {
        al.push(
            SERVER_ROOM_ID.into_inner().into(),
            AuditLogEntryType::UserUpdate { changes },
        )
        .success();
    }

    Ok(routes::user_update::Response { user })
}

#[handler(routes::user_delete)]
pub async fn delete(
    req: Req<routes::user_delete::Endpoint>,
) -> Result<routes::user_delete::Response> {
    let srv = req.services();
    let auth_user = req.identity().ensure_user()?;

    let target_user_id = req.inner().user_id.clone().local_unwrap_or(auth_user.id)?;

    let mut perms = srv
        .perms
        .for_room3(Some(auth_user.id), SERVER_ROOM_ID)
        .await
        .cast_internal()?
        .ensure_view()?;
    if auth_user.id != target_user_id {
        perms.needs(Permission::UserManage);
    } else {
        perms.needs(Permission::UserManageSelf);
    }
    perms.check()?;

    let user = srv
        .users
        .get(target_user_id, Some(auth_user.id))
        .await
        .cast_internal()?;

    // keep the user's avatar/banner media linked so it can be restored on undelete
    let mut data = req.globals().begin().await.cast_internal()?;
    data.user_delete(target_user_id).await.cast_internal()?;
    data.commit().await.cast_internal()?;

    srv.users.invalidate(target_user_id).await;
    req.globals()
        .messaging()
        .broadcast_global(MessageSync::UserDelete { id: target_user_id })
        .await
        .cast_internal()?;

    req.audit_log()
        .push(
            target_user_id.into_inner().into(),
            AuditLogEntryType::UserDelete {
                user_id: target_user_id,
                changes: Changes::new()
                    .remove("name", &user.name)
                    .remove("description", &user.description)
                    .remove("avatar", &user.avatar)
                    .remove("banner", &user.banner)
                    .build(),
            },
        )
        .success();

    Ok(routes::user_delete::Response {})
}

#[handler(routes::user_undelete)]
pub async fn undelete(
    req: Req<routes::user_undelete::Endpoint>,
) -> Result<routes::user_undelete::Response> {
    let srv = req.services();
    let auth_user = req.identity().ensure_user()?;
    auth_user.ensure_unsuspended()?;

    let target_user_id = req.inner().user_id.clone().local_unwrap_or(auth_user.id)?;

    srv.perms
        .for_room3(Some(auth_user.id), SERVER_ROOM_ID)
        .await
        .cast_internal()?
        .ensure_view()?
        .needs(Permission::UserManage)
        .check()?;

    let mut data = req.globals().begin().await.cast_internal()?;
    data.user_undelete(target_user_id).await.cast_internal()?;
    data.commit().await.cast_internal()?;

    srv.users.invalidate(target_user_id).await;
    let user = srv
        .users
        .get(target_user_id, Some(auth_user.id))
        .await
        .cast_internal()?;
    req.globals()
        .messaging()
        .broadcast_global(MessageSync::UserCreate { user: user.clone() })
        .await
        .cast_internal()?;

    req.audit_log()
        .push(
            SERVER_ROOM_ID.into_inner().into(),
            AuditLogEntryType::UserUndelete {
                user_id: target_user_id,
            },
        )
        .success();

    Ok(routes::user_undelete::Response {})
}

#[handler(routes::user_audit_logs)]
pub async fn audit_logs(
    req: Req<routes::user_audit_logs::Endpoint>,
) -> Result<routes::user_audit_logs::Response> {
    let srv = req.services();
    let auth_user_id = req.identity().ensure_user()?.id;

    let body = req.into_inner();
    let target_user_id = body.user_id.local_unwrap_or(auth_user_id)?;
    if auth_user_id != target_user_id {
        return Err(ApiError::from_code(ErrorCode::UnknownUser).into());
    }

    let logs = srv
        .audit_logs
        .list(
            target_user_id.into_inner().into(),
            body.pagination,
            body.filter,
        )
        .await
        .cast_internal()?;

    Ok(routes::user_audit_logs::Response { logs })
}

#[handler(routes::user_create)]
pub async fn create(
    req: Req<routes::user_create::Endpoint>,
) -> Result<routes::user_create::Response> {
    let globals = req.globals();
    let srv = req.services();
    let session = req.identity().ensure_session()?.clone();
    let create = req.inner().create.clone();

    let registered_at = if globals.config().require_server_invite {
        None
    } else {
        Some(Time::now_utc())
    };

    let parent_id = match &create.invite_code {
        Some(invite_code) => {
            let mut data = globals.begin_read().await.cast_internal()?;
            let invite = data.invite_get(invite_code.clone()).await.cast_internal()?;
            Some(invite.invite.creator_id)
        }
        None => None,
    };

    let mut data = globals.begin().await.cast_internal()?;
    let mut user = data
        .user_create(DbUserCreate {
            id: None,
            parent_id,
            name: create.name,
            description: create.description,
            puppet: None,
            registered_at,
            system: false,
            bot: false,
            remote: None,
        })
        .await
        .cast_internal()?;
    // parent is only visible to managers, which this new user is not
    // NOTE: maybe i want to expose parent_id to users anyways? ie. make it semi-public?
    // or i could make it more restricted, ie. don't let people see who created an application?
    user.parent_id = None;

    data.room_member_put(SERVER_ROOM_ID, user.id, None, RoomMemberPut::default())
        .await
        .cast_internal()?;
    data.session_set_status(session.id, SessionStatus::Authorized { user_id: user.id })
        .await
        .cast_internal()?;
    data.commit().await.cast_internal()?;

    srv.perms.invalidate_room(user.id, SERVER_ROOM_ID).await;
    srv.sessions.invalidate(session.id).await;

    let updated_session = srv.sessions.get(session.id).await.cast_internal()?;
    globals
        .messaging()
        .broadcast_global(MessageSync::SessionCreate {
            session: (*updated_session).clone(),
        })
        .await
        .cast_internal()?;

    req.audit_log_as(user.id, &session)
        .push(
            user.id.into_inner().into(),
            AuditLogEntryType::SessionLogin {
                user_id: user.id,
                session_id: updated_session.id,
            },
        )
        .success();

    Ok(routes::user_create::Response { user })
}

#[handler(routes::user_suspend)]
pub async fn suspend(
    req: Req<routes::user_suspend::Endpoint>,
) -> Result<routes::user_suspend::Response> {
    let srv = req.services();
    let auth_user = req.identity().ensure_user()?;
    auth_user.ensure_unsuspended()?;

    let body = req.inner();
    let auth_user_id = auth_user.id;
    let target_user_id = body.user_id.clone().local_unwrap_or(auth_user_id)?;

    if target_user_id != auth_user_id {
        srv.perms
            .for_room3(Some(auth_user_id), SERVER_ROOM_ID)
            .await
            .cast_internal()?
            .ensure_view()?
            .needs(Permission::MemberBan)
            .check()?;
    }

    let mut data = req.globals().begin().await.cast_internal()?;
    data.user_suspended(
        target_user_id,
        Some(Suspended {
            created_at: Time::now_utc(),
            expires_at: body.suspend.expires_at,
            reason: body.reason.clone(),
        }),
    )
    .await
    .cast_internal()?;
    data.commit().await.cast_internal()?;

    req.audit_log()
        .push(
            SERVER_ROOM_ID.into_inner().into(),
            AuditLogEntryType::UserSuspend {
                expires_at: body.suspend.expires_at,
                user_id: target_user_id,
            },
        )
        .success();

    srv.users.invalidate(target_user_id).await;
    let user = srv
        .users
        .get(target_user_id, Some(auth_user_id))
        .await
        .cast_internal()?;
    req.globals()
        .messaging()
        .broadcast_global(MessageSync::UserUpdate { user: user.clone() })
        .await
        .cast_internal()?;

    Ok(routes::user_suspend::Response { user })
}

#[handler(routes::user_unsuspend)]
pub async fn unsuspend(
    req: Req<routes::user_unsuspend::Endpoint>,
) -> Result<routes::user_unsuspend::Response> {
    let srv = req.services();
    let auth_user = req.identity().ensure_user()?;
    auth_user.ensure_unsuspended()?;

    let auth_user_id = auth_user.id;
    let target_user_id = req.inner().user_id.clone().local_unwrap_or(auth_user_id)?;

    srv.perms
        .for_room3(Some(auth_user_id), SERVER_ROOM_ID)
        .await
        .cast_internal()?
        .ensure_view()?
        .needs(Permission::MemberBan)
        .check()?;

    let mut data = req.globals().begin().await.cast_internal()?;
    data.user_suspended(target_user_id, None)
        .await
        .cast_internal()?;
    data.commit().await.cast_internal()?;

    req.audit_log()
        .push(
            SERVER_ROOM_ID.into_inner().into(),
            AuditLogEntryType::UserUnsuspend {
                user_id: target_user_id,
            },
        )
        .success();

    srv.users.invalidate(target_user_id).await;
    let user = srv
        .users
        .get(target_user_id, Some(auth_user_id))
        .await
        .cast_internal()?;
    req.globals()
        .messaging()
        .broadcast_global(MessageSync::UserUpdate { user: user.clone() })
        .await
        .cast_internal()?;

    Ok(routes::user_unsuspend::Response { user })
}

#[handler(routes::user_presence_set)]
pub async fn presence_set(
    req: Req<routes::user_presence_set::Endpoint>,
) -> Result<routes::user_presence_set::Response> {
    let srv = req.services();
    let auth_user = req.identity().ensure_user()?;
    auth_user.ensure_unsuspended()?;

    let body = req.inner();
    let target_user_id = body.user_id.clone().local_unwrap_or(auth_user.id)?;
    if auth_user.id != target_user_id {
        return Err(ApiError::from_code(ErrorCode::MissingPermissions).into());
    }

    srv.presence
        .set_manually(target_user_id, body.presence.clone());

    Ok(routes::user_presence_set::Response {})
}

#[handler(routes::user_list)]
pub async fn list(req: Req<routes::user_list::Endpoint>) -> Result<routes::user_list::Response> {
    let globals = req.globals();
    let srv = req.services();
    let auth_user = req.identity().ensure_user()?;
    auth_user.ensure_unsuspended()?;
    let auth_user_id = auth_user.id;

    let mut perms = srv
        .perms
        .for_room3(Some(auth_user_id), SERVER_ROOM_ID)
        .await
        .cast_internal()?
        .ensure_view()?;
    if !perms.has_any(&[Permission::UserManage, Permission::ServerOversee]) {
        perms.needs(Permission::UserManage);
    }
    perms.check()?;
    let can_manage = perms.has(Permission::UserManage);

    let (pagination, filter) = req.into_inner().query.into_parts();
    let mut data = globals.begin_read().await.cast_internal()?;
    let mut users = data.user_list(pagination, filter).await.cast_internal()?;

    for user in &mut users.items {
        user.emails = Some(data.user_email_list(user.id).await.cast_internal()?);
        if !can_manage && !user.bot && user.puppet.is_none() {
            user.parent_id = None;
        }
    }

    Ok(routes::user_list::Response { users })
}

#[handler(routes::user_search)]
pub async fn search(
    req: Req<routes::user_search::Endpoint>,
) -> Result<routes::user_search::Response> {
    let srv = req.services();
    let auth_user = req.identity().ensure_user()?;

    let mut perms = srv
        .perms
        .for_room3(Some(auth_user.id), SERVER_ROOM_ID)
        .await
        .cast_internal()?
        .ensure_view()?;
    if !perms.has_any(&[Permission::UserManage, Permission::ServerOversee]) {
        perms.needs(Permission::UserManage);
    }
    perms.check()?;

    let body = req.into_inner();
    let results = srv.search.search_users(body.search).await.cast_internal()?;

    Ok(routes::user_search::Response { results })
}

export_routes!(
    get,
    update,
    delete,
    undelete,
    audit_logs,
    create,
    suspend,
    unsuspend,
    presence_set,
    list,
    search
);
