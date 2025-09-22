use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::util::{Changes, Time};
use common::v1::types::{
    AuditLogEntry, AuditLogEntryId, AuditLogEntryType, Invite, InviteCode, InviteCreate,
    InvitePatch, InviteTarget, InviteTargetId, InviteWithMetadata, MessageSync, PaginationQuery,
    PaginationResponse, Permission, RoomId, RoomMemberOrigin, RoomMemberPut, SERVER_ROOM_ID,
};
use http::StatusCode;
use nanoid::nanoid;
use serde::Serialize;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::error::Result;
use crate::routes::auth::fetch_auth_state;
use crate::{Error, ServerState};

use super::util::{Auth, HeaderReason};

/// Invite delete
#[utoipa::path(
    delete,
    path = "/invite/{invite_code}",
    params(
        ("invite_code", description = "The code identifying this invite"),
    ),
    tags = ["invite", "badge.perm-opt.InviteManage"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
async fn invite_delete(
    Path(code): Path<InviteCode>,
    Auth(auth_user): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let d = s.data();
    let invite = d.invite_select(code.clone()).await?;
    let (has_perm, id_target) = match invite.invite.target {
        InviteTarget::Room { room } => (
            s.services()
                .perms
                .for_room(auth_user.id, room.id)
                .await?
                .has(Permission::InviteManage),
            InviteTargetId::Room { room_id: room.id },
        ),
        InviteTarget::Thread { room, thread } => (
            s.services()
                .perms
                .for_thread(auth_user.id, thread.id)
                .await?
                .has(Permission::InviteManage),
            InviteTargetId::Thread {
                room_id: room.id,
                thread_id: thread.id,
            },
        ),
        InviteTarget::Server => (
            s.services()
                .perms
                .for_room(auth_user.id, SERVER_ROOM_ID)
                .await?
                .has(Permission::InviteManage),
            InviteTargetId::Server,
        ),
    };
    let can_delete = auth_user.id == invite.invite.creator_id || has_perm;
    if can_delete {
        d.invite_delete(code.clone()).await?;
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id: match id_target {
                InviteTargetId::Room { room_id } => room_id,
                InviteTargetId::Thread { room_id, .. } => room_id,
                InviteTargetId::Server => SERVER_ROOM_ID,
            },
            user_id: auth_user.id,
            session_id: None,
            reason: reason.clone(),
            ty: AuditLogEntryType::InviteDelete { code: code.clone() },
        })
        .await?;
        match id_target {
            InviteTargetId::Room { room_id } => {
                s.broadcast_room(
                    room_id,
                    auth_user.id,
                    MessageSync::InviteDelete {
                        code,
                        target: id_target,
                    },
                )
                .await?;
            }
            InviteTargetId::Thread { thread_id, .. } => {
                s.broadcast_thread(
                    thread_id,
                    auth_user.id,
                    MessageSync::InviteDelete {
                        code,
                        target: id_target,
                    },
                )
                .await?;
            }
            InviteTargetId::Server => {
                s.broadcast_room(
                    SERVER_ROOM_ID,
                    auth_user.id,
                    MessageSync::InviteDelete {
                        code,
                        target: id_target,
                    },
                )
                .await?;
            }
        };
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Invite resolve
#[utoipa::path(
    get,
    path = "/invite/{invite_code}",
    params(
        ("invite_code", description = "The code identifying this invite"),
    ),
    tags = ["invite"],
    responses(
        (status = OK, body = Invite, description = "success"),
        (status = OK, body = InviteWithMetadata, description = "success with metadata"),
    )
)]
async fn invite_resolve(
    Path(code): Path<InviteCode>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let s = s.services();
    let invite = d.invite_select(code).await?;
    if invite.invite.creator_id == auth_user.id {
        return Ok(Json(invite).into_response());
    }
    let should_strip = match &invite.invite.target {
        InviteTarget::Room { room } => {
            let perms = s.perms.for_room(auth_user.id, room.id).await?;
            !perms.has(Permission::InviteManage)
        }
        InviteTarget::Thread { room: _, thread } => {
            let perms = s.perms.for_thread(auth_user.id, thread.id).await?;
            !perms.has(Permission::InviteManage)
        }
        InviteTarget::Server => {
            let perms = s.perms.for_room(auth_user.id, SERVER_ROOM_ID).await?;
            !perms.has(Permission::InviteManage)
        }
    };
    if should_strip {
        Ok(Json(invite.strip_metadata()).into_response())
    } else {
        Ok(Json(invite).into_response())
    }
}

/// Invite use
///
/// - A room invite will add the user to the room
/// - A thread invite will currently do the same thing as a room invite
/// - A server invite will upgrade the user to a full account
///
/// using a server invite may require the guest to first
///
/// - solve an antispam challenge, such as a captcha
/// - add an authentication method, such as (email && password) || oauth

#[utoipa::path(
    post,
    path = "/invite/{invite_code}",
    params(
        ("invite_code", description = "The code identifying this invite"),
    ),
    tags = ["invite"],
    responses(
        (status = OK, description = "success"),
    )
)]
async fn invite_use(
    Path(code): Path<InviteCode>,
    Auth(user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let srv = s.services();
    let invite = d.invite_select(code.clone()).await?;
    if invite.is_dead() {
        return Err(Error::NotFound);
    }
    match invite.invite.target {
        InviteTarget::Thread { room, .. } | InviteTarget::Room { room } => {
            if d.room_ban_get(room.id, user.id).await.is_ok() {
                return Err(Error::BadStatic("banned"));
            }
            let origin = RoomMemberOrigin::Invite {
                code: invite.invite.code,
                inviter: invite.invite.creator_id,
            };
            d.room_member_put(room.id, user.id, Some(origin), RoomMemberPut::default())
                .await?;
            let member = d.room_member_get(room.id, user.id).await?;
            srv.perms.invalidate_room(user.id, room.id).await;
            srv.perms.invalidate_is_mutual(user.id);
            let room_id = room.id;
            // FIXME: don't send RoomCreate to *everyone* when someone joins, just the joining user
            s.broadcast_room(room_id, user.id, MessageSync::RoomCreate { room })
                .await?;
            s.broadcast_room(room_id, user.id, MessageSync::RoomMemberUpsert { member })
                .await?;
        }
        InviteTarget::Server => {
            let srv = s.services();
            let user = srv.users.get(user.id).await?;
            if user.registered_at.is_some() {
                return Err(Error::BadStatic("User is not a guest account"));
            }
            let auth_state = fetch_auth_state(&s, user.id).await?;
            if !auth_state.can_login() {
                // make sure to prevent people from creating accounts they can't log into
                return Err(Error::BadStatic("add an auth method first"));
            }
            d.user_set_registered(user.id, Some(Time::now_utc()), Some(invite.invite.code.0))
                .await?;
            d.room_member_put(SERVER_ROOM_ID, user.id, None, RoomMemberPut::default())
                .await?;
            srv.users.invalidate(user.id).await;
            let updated_user = srv.users.get(user.id).await?;
            s.audit_log_append(AuditLogEntry {
                id: AuditLogEntryId::new(),
                room_id: SERVER_ROOM_ID,
                user_id: user.id,
                session_id: None,
                reason,
                ty: AuditLogEntryType::UserRegistered { user_id: user.id },
            })
            .await?;
            s.broadcast(MessageSync::UserUpdate { user: updated_user })?;
        }
    }
    d.invite_incr_use(code).await?;
    Ok(())
}

/// Invite room create
///
/// Create an invite that goes to a room
#[utoipa::path(
    post,
    path = "/room/{room_id}/invite",
    params(
        ("room_id", description = "Room id"),
    ),
    tags = ["invite", "badge.perm.InviteCreate"],
    responses(
        (status = OK, body = Invite, description = "success"),
    )
)]
async fn invite_room_create(
    Path(room_id): Path<RoomId>,
    Auth(auth_user): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<InviteCreate>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;

    let d = s.data();
    let perms = s.services.perms.for_room(auth_user.id, room_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::InviteCreate)?;

    if room_id == SERVER_ROOM_ID {
        return Err(Error::BadStatic("You can't create an invite for the server room. Use the make-admin subcommand in the cli instead."));
    }

    let alphabet: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
        .chars()
        .collect();
    let code = InviteCode(nanoid!(8, &alphabet));
    d.invite_insert_room(
        room_id,
        auth_user.id,
        code.clone(),
        json.expires_at,
        json.max_uses,
    )
    .await?;
    let invite = d.invite_select(code).await?;

    let changes = Changes::new()
        .add("code", &invite.invite.code)
        .add("description", &invite.invite.description)
        .add("expires_at", &invite.invite.expires_at)
        .add("max_uses", &invite.max_uses)
        .build();

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth_user.id,
        session_id: None,
        reason: reason.clone(),
        ty: AuditLogEntryType::InviteCreate { changes },
    })
    .await?;

    s.broadcast_room(
        room_id,
        auth_user.id,
        MessageSync::InviteCreate {
            invite: invite.clone(),
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(invite)))
}

#[derive(Serialize)]
#[serde(untagged)]
enum InviteWithPotentialMetadata {
    Invite(Invite),
    InviteWithMetadata(InviteWithMetadata),
}

/// Invite room list
///
/// List invites that go to a room
#[utoipa::path(
    get,
    path = "/room/{room_id}/invite",
    params(
        PaginationQuery<InviteCode>,
        ("room_id", description = "Room id"),
    ),
    tags = ["invite"],
    responses(
        (status = OK, body = PaginationResponse<Invite>, description = "success"),
    )
)]
async fn invite_room_list(
    Path(room_id): Path<RoomId>,
    Query(paginate): Query<PaginationQuery<InviteCode>>,
    Auth(user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let perms = s.services.perms.for_room(user.id, room_id).await?;
    perms.ensure_view()?;
    let res = d.invite_list_room(room_id, paginate).await?;
    let items: Vec<_> = res
        .items
        .into_iter()
        .map(|i| {
            if i.invite.creator_id != user.id && !perms.has(Permission::InviteManage) {
                InviteWithPotentialMetadata::Invite(i.strip_metadata())
            } else {
                InviteWithPotentialMetadata::InviteWithMetadata(i)
            }
        })
        .collect();
    let cursor = items.last().map(|i| match i {
        InviteWithPotentialMetadata::Invite(invite) => invite.code.0.clone(),
        InviteWithPotentialMetadata::InviteWithMetadata(invite_with_metadata) => {
            invite_with_metadata.invite.code.0.clone()
        }
    });
    let res = PaginationResponse {
        items,
        total: res.total,
        has_more: res.has_more,
        cursor,
    };
    Ok(Json(res))
}

/// Invite patch
///
/// Edit an invite
#[utoipa::path(
    patch,
    path = "/invite/{invite_code}",
    params(
        ("invite_code", description = "The code identifying this invite"),
    ),
    tags = ["invite", "badge.perm-opt.InviteManage"],
    responses(
        (status = NOT_MODIFIED, description = "not modified"),
        (status = OK, body = Invite, description = "success"),
    )
)]
async fn invite_patch(
    Path(code): Path<InviteCode>,
    Auth(user): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
    Json(patch): Json<InvitePatch>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let start_invite = d.invite_select(code.clone()).await?;

    let (has_perm, _id_target) = match start_invite.invite.target {
        InviteTarget::Room { room } => (
            s.services()
                .perms
                .for_room(user.id, room.id)
                .await?
                .has(Permission::InviteManage),
            InviteTargetId::Room { room_id: room.id },
        ),
        InviteTarget::Thread { room, thread } => (
            s.services()
                .perms
                .for_thread(user.id, thread.id)
                .await?
                .has(Permission::InviteManage),
            InviteTargetId::Thread {
                room_id: room.id,
                thread_id: thread.id,
            },
        ),
        InviteTarget::Server => (
            s.services()
                .perms
                .for_room(user.id, SERVER_ROOM_ID)
                .await?
                .has(Permission::InviteManage),
            InviteTargetId::Server,
        ),
    };

    let can_patch = user.id == start_invite.invite.creator_id || has_perm;
    if !can_patch {
        return Err(Error::MissingPermissions);
    }

    let updated_invite = d.invite_update(code.clone(), patch).await?;

    let changes = Changes::new()
        .change(
            "description",
            &start_invite.invite.description,
            &updated_invite.invite.description,
        )
        .change(
            "expires_at",
            &start_invite.invite.expires_at,
            &updated_invite.invite.expires_at,
        )
        .change("max_uses", &start_invite.max_uses, &updated_invite.max_uses)
        .build();

    let room_id = match &updated_invite.invite.target {
        InviteTarget::Room { room } => Some(room.id),
        InviteTarget::Thread { room, .. } => Some(room.id),
        InviteTarget::Server => None,
    };
    if let Some(room_id) = room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: user.id,
            session_id: None,
            reason,
            ty: AuditLogEntryType::InviteUpdate { changes },
        })
        .await?;
    }

    // TODO: Check if any actual changes were made and return NOT_MODIFIED if not.
    // This would require comparing `invite` and `updated_invite`.

    s.broadcast(MessageSync::InviteUpdate {
        invite: updated_invite.clone(),
    })?;

    Ok((StatusCode::OK, Json(updated_invite)))
}

/// Invite server create
///
/// Create an invite that allows registration on a server.
#[utoipa::path(
    post,
    path = "/server/invite",
    tags = ["invite", "badge.perm.InviteCreate"],
    responses((status = OK, body = Invite, description = "success")),
)]
async fn invite_server_create(
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<InviteCreate>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;

    let d = s.data();
    let srv = s.services();
    let user = srv.users.get(auth_user.id).await?;
    if user.registered_at.is_none() {
        return Err(Error::BadStatic("Guest users cannot create server invites"));
    }

    let perms = srv.perms.for_room(user.id, SERVER_ROOM_ID).await?;
    perms.ensure(Permission::InviteCreate)?;

    let alphabet: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
        .chars()
        .collect();
    let code = InviteCode(nanoid!(8, &alphabet));
    d.invite_insert_server(user.id, code.clone(), json.expires_at, json.max_uses)
        .await?;
    let invite = d.invite_select(code).await?;

    let changes = Changes::new()
        .add("code", &invite.invite.code)
        .add("description", &invite.invite.description)
        .add("expires_at", &invite.invite.expires_at)
        .add("max_uses", &invite.max_uses)
        .build();

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: SERVER_ROOM_ID,
        user_id: user.id,
        session_id: None,
        reason: reason.clone(),
        ty: AuditLogEntryType::InviteCreate { changes },
    })
    .await?;

    s.broadcast_room(
        SERVER_ROOM_ID,
        user.id,
        MessageSync::InviteCreate {
            invite: invite.clone(),
        },
    )
    .await?;

    Ok((StatusCode::CREATED, Json(invite)))
}

/// Invite server list
///
/// List invites that allow registration on a server
#[utoipa::path(
    get,
    path = "/server/invite",
    params(
        PaginationQuery<InviteCode>,
    ),
    tags = ["invite"],
    responses(
        (status = OK, body = PaginationResponse<Invite>, description = "success"),
    )
)]
async fn invite_server_list(
    Query(paginate): Query<PaginationQuery<InviteCode>>,
    Auth(user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let d = s.data();
    let user = srv.users.get(user.id).await?;
    if user.registered_at.is_none() {
        return Err(Error::BadStatic("Guest users cannot list server invites"));
    }

    // TODO: let users list their own server invites
    let perms = srv.perms.for_room(user.id, SERVER_ROOM_ID).await?;
    perms.ensure(Permission::InviteManage)?;

    let res = d.invite_list_server(paginate).await?;
    Ok(Json(res))
}

/// Invite user create
///
/// Creates an invite that adds this user as a friend when used
#[utoipa::path(
    post,
    path = "/user/{user_id}/invite",
    params(("user_id", description = "User id")),
    tags = ["invite"],
    responses((status = OK, body = Invite, description = "success")),
)]
async fn invite_user_create(
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
    HeaderReason(_reason): HeaderReason,
    Json(_json): Json<InviteCreate>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Invite user list
#[utoipa::path(
    get,
    path = "/user/{user_id}/invite",
    params(
        PaginationQuery<InviteCode>,
        ("user_id", description = "User id"),
    ),
    tags = ["invite"],
    responses(
        (status = OK, body = PaginationResponse<Invite>, description = "success"),
    )
)]
async fn invite_user_list(
    Query(_paginate): Query<PaginationQuery<InviteCode>>,
    Auth(_user): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

// TODO: consider merging invite_server_* with invite_room_*?
pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(invite_delete))
        .routes(routes!(invite_resolve))
        .routes(routes!(invite_patch))
        .routes(routes!(invite_use))
        .routes(routes!(invite_room_create))
        .routes(routes!(invite_room_list))
        .routes(routes!(invite_server_create))
        .routes(routes!(invite_server_list))
        .routes(routes!(invite_user_create))
        .routes(routes!(invite_user_list))
}
