use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::misc::UserIdReq;
use common::v1::types::util::{Changes, Time};
use common::v1::types::{
    AuditLogEntry, AuditLogEntryId, AuditLogEntryType, Invite, InviteCode, InviteCreate,
    InvitePatch, InviteTarget, InviteTargetId, InviteWithMetadata, MessageSync, PaginationQuery,
    PaginationResponse, Permission, RelationshipPatch, RelationshipType, RoomId, RoomMemberOrigin,
    RoomMemberPut, RoomMembership, ThreadId, SERVER_ROOM_ID,
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
        InviteTarget::Room { room, thread } => (
            s.services()
                .perms
                .for_room(auth_user.id, room.id)
                .await?
                .has(Permission::InviteManage),
            InviteTargetId::Room {
                room_id: room.id,
                thread_id: thread.map(|t| t.id),
            },
        ),
        InviteTarget::Gdm { thread } => (
            s.services()
                .perms
                .for_thread(auth_user.id, thread.id)
                .await?
                .has(Permission::InviteManage),
            InviteTargetId::Gdm {
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
        InviteTarget::User { user } => (false, InviteTargetId::User { user_id: user.id }),
    };
    let can_delete = auth_user.id == invite.invite.creator_id || has_perm;
    if can_delete {
        d.invite_delete(code.clone()).await?;
        let room_id = match id_target {
            InviteTargetId::Room { room_id, .. } => Some(room_id),
            InviteTargetId::Gdm { .. } => None,
            InviteTargetId::Server => Some(SERVER_ROOM_ID),
            InviteTargetId::User { user_id } => Some((*user_id).into()),
        };
        if let Some(room_id) = room_id {
            s.audit_log_append(AuditLogEntry {
                id: AuditLogEntryId::new(),
                room_id,
                user_id: auth_user.id,
                session_id: None,
                reason: reason.clone(),
                ty: AuditLogEntryType::InviteDelete { code: code.clone() },
            })
            .await?;
        }
        match id_target {
            InviteTargetId::Room { room_id, .. } => {
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
            InviteTargetId::Gdm { thread_id } => {
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
            InviteTargetId::User { .. } => {
                s.broadcast(MessageSync::InviteDelete {
                    code,
                    target: id_target,
                })?;
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
        InviteTarget::Room { room, .. } => {
            let perms = s.perms.for_room(auth_user.id, room.id).await?;
            !perms.has(Permission::InviteManage)
        }
        InviteTarget::Gdm { thread } => {
            let perms = s.perms.for_thread(auth_user.id, thread.id).await?;
            !perms.has(Permission::InviteManage)
        }
        InviteTarget::Server => {
            let perms = s.perms.for_room(auth_user.id, SERVER_ROOM_ID).await?;
            !perms.has(Permission::InviteManage)
        }
        InviteTarget::User { user: _ } => auth_user.id != invite.invite.creator_id,
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
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let srv = s.services();
    let invite = d.invite_select(code.clone()).await?;
    if invite.is_dead() {
        return Err(Error::NotFound);
    }
    match &invite.invite.target {
        InviteTarget::Room { room, .. } => {
            if let Ok(ban) = d.room_ban_get(room.id, auth_user.id).await {
                if let Some(expires_at) = ban.expires_at {
                    if expires_at > Time::now_utc() {
                        return Err(Error::BadStatic("banned"));
                    }
                } else {
                    return Err(Error::BadStatic("banned"));
                }
            }

            let origin = RoomMemberOrigin::Invite {
                code: invite.invite.code,
                inviter: invite.invite.creator_id,
            };
            let existing = d.room_member_get(room.id, auth_user.id).await;
            if existing.is_ok_and(|e| e.membership == RoomMembership::Join) {
                return Ok(());
            }

            d.room_member_put(
                room.id,
                auth_user.id,
                Some(origin),
                RoomMemberPut::default(),
            )
            .await?;
            let member = d.room_member_get(room.id, auth_user.id).await?;
            srv.perms.invalidate_room(auth_user.id, room.id).await;
            srv.perms.invalidate_is_mutual(auth_user.id);
            let room_id = room.id;
            // FIXME: don't send RoomCreate to *everyone* when someone joins, just the joining user
            s.broadcast_room(
                room_id,
                auth_user.id,
                MessageSync::RoomCreate { room: room.clone() },
            )
            .await?;
            s.broadcast_room(
                room_id,
                auth_user.id,
                MessageSync::RoomMemberUpsert { member },
            )
            .await?;
        }
        InviteTarget::Gdm { .. } => todo!(),
        InviteTarget::Server => {
            let srv = s.services();
            let user = srv.users.get(auth_user.id, None).await?;
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
            let updated_user = srv.users.get(user.id, None).await?;
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
        InviteTarget::User { user } => {
            d.user_relationship_edit(
                auth_user.id,
                user.id,
                RelationshipPatch {
                    note: None,
                    petname: None,
                    ignore: None,
                    relation: Some(Some(RelationshipType::Friend)),
                },
            )
            .await?;
            d.user_relationship_edit(
                user.id,
                auth_user.id,
                RelationshipPatch {
                    note: None,
                    petname: None,
                    ignore: None,
                    relation: Some(Some(RelationshipType::Friend)),
                },
            )
            .await?;

            if let Some(rel) = d.user_relationship_get(auth_user.id, user.id).await? {
                s.broadcast(MessageSync::RelationshipUpsert {
                    user_id: auth_user.id,
                    target_user_id: user.id,
                    relationship: rel,
                })?;
            }

            if let Some(rel) = d.user_relationship_get(user.id, auth_user.id).await? {
                s.broadcast(MessageSync::RelationshipUpsert {
                    user_id: user.id,
                    target_user_id: auth_user.id,
                    relationship: rel,
                })?;
            }

            // TODO: should i append to user audit logs here?
        }
    }
    d.invite_incr_use(code).await?;

    // TODO: send welcome message to gdm
    let room_id = match &invite.invite.target {
        InviteTarget::Room { room, .. } => room.id,
        InviteTarget::Gdm { .. } => return Ok(()),
        InviteTarget::Server => SERVER_ROOM_ID,
        InviteTarget::User { .. } => return Ok(()),
    };
    srv.rooms
        .send_welcome_message(room_id, auth_user.id)
        .await?;

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

/// Invite thread create
///
/// Create an invite that goes to a thread
#[utoipa::path(
    post,
    path = "/thread/{thread_id}/invite",
    params(
        ("thread_id", description = "Thread id"),
    ),
    tags = ["invite", "badge.perm.InviteCreate"],
    responses(
        (status = OK, body = Invite, description = "success"),
    )
)]
async fn invite_thread_create(
    Path(_thread_id): Path<ThreadId>,
    Auth(_auth_user): Auth,
    HeaderReason(_reason): HeaderReason,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<InviteCreate>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Invite thread list
///
/// List invites that go to a thread
#[utoipa::path(
    get,
    path = "/thread/{thread_id}/invite",
    params(
        PaginationQuery<InviteCode>,
        ("thread_id", description = "Thread id"),
    ),
    tags = ["invite"],
    responses(
        (status = OK, body = PaginationResponse<Invite>, description = "success"),
    )
)]
async fn invite_thread_list(
    Path(_thread_id): Path<ThreadId>,
    Query(_paginate): Query<PaginationQuery<InviteCode>>,
    Auth(_user): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
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
    Auth(auth_user): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
    Json(patch): Json<InvitePatch>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let start_invite = d.invite_select(code.clone()).await?;

    let (has_perm, _id_target) = match start_invite.invite.target {
        InviteTarget::Room { room, thread } => (
            s.services()
                .perms
                .for_room(auth_user.id, room.id)
                .await?
                .has(Permission::InviteManage),
            InviteTargetId::Room {
                room_id: room.id,
                thread_id: thread.map(|t| t.id),
            },
        ),
        InviteTarget::Gdm { thread } => (
            s.services()
                .perms
                .for_thread(auth_user.id, thread.id)
                .await?
                .has(Permission::InviteManage),
            InviteTargetId::Gdm {
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
        InviteTarget::User { user: _ } => (
            auth_user.id == start_invite.invite.creator_id,
            InviteTargetId::User {
                user_id: auth_user.id,
            },
        ),
    };

    let can_patch = auth_user.id == start_invite.invite.creator_id || has_perm;
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
        InviteTarget::Room { room, .. } => Some(room.id),
        InviteTarget::Gdm { .. } => None,
        InviteTarget::Server => Some(SERVER_ROOM_ID),
        InviteTarget::User { user } => Some((*user.id).into()),
    };
    if let Some(room_id) = room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth_user.id,
            session_id: None,
            reason,
            ty: AuditLogEntryType::InviteUpdate { changes },
        })
        .await?;
    }

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
    let user = srv.users.get(auth_user.id, None).await?;
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
    let user = srv.users.get(user.id, None).await?;
    if user.registered_at.is_none() {
        return Err(Error::BadStatic("Guest users cannot list server invites"));
    }

    let perms = srv.perms.for_room(user.id, SERVER_ROOM_ID).await?;
    let res = if perms.has(Permission::InviteManage) {
        d.invite_list_server(paginate).await?
    } else {
        d.invite_list_server_by_creator(user.id, paginate).await?
    };
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
    Path(target_user_id): Path<UserIdReq>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<InviteCreate>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;

    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user.id,
        UserIdReq::UserId(id) => id,
    };

    if auth_user.id != target_user_id {
        return Err(Error::NotFound);
    }

    let d = s.data();

    let alphabet: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
        .chars()
        .collect();
    let code = InviteCode(nanoid!(8, &alphabet));
    d.invite_insert_user(
        auth_user.id,
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
        room_id: auth_user.id.into_inner().into(),
        user_id: auth_user.id,
        session_id: None,
        reason: reason.clone(),
        ty: AuditLogEntryType::InviteCreate { changes },
    })
    .await?;

    s.broadcast(MessageSync::InviteCreate {
        invite: invite.clone(),
    })?;

    Ok((StatusCode::CREATED, Json(invite)))
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
    ),
)]
async fn invite_user_list(
    Path(target_user_id): Path<UserIdReq>,
    Query(paginate): Query<PaginationQuery<InviteCode>>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user.id,
        UserIdReq::UserId(id) => id,
    };

    if auth_user.id != target_user_id {
        return Err(Error::NotFound);
    }

    let d = s.data();
    let res = d.invite_list_user(target_user_id, paginate).await?;

    let items: Vec<_> = res
        .items
        .into_iter()
        .map(|i| {
            if i.invite.creator_id != auth_user.id {
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

// TODO: consider merging invite_server_* with invite_room_*?
pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(invite_delete))
        .routes(routes!(invite_resolve))
        .routes(routes!(invite_patch))
        .routes(routes!(invite_use))
        .routes(routes!(invite_room_create))
        .routes(routes!(invite_room_list))
        .routes(routes!(invite_thread_create))
        .routes(routes!(invite_thread_list))
        .routes(routes!(invite_server_create))
        .routes(routes!(invite_server_list))
        .routes(routes!(invite_user_create))
        .routes(routes!(invite_user_list))
}
