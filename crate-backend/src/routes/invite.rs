use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::misc::UserIdReq;
use common::v1::types::util::{Changes, Time};
use common::v1::types::{
    AuditLogEntry, AuditLogEntryId, AuditLogEntryType, ChannelId, ChannelType, Invite, InviteCode,
    InviteCreate, InvitePatch, InviteTarget, InviteTargetId, InviteWithMetadata, MessageSync,
    PaginationQuery, PaginationResponse, Permission, RelationshipPatch, RelationshipType, RoomId,
    RoomMemberOrigin, RoomMemberPut, RoomMembership, SERVER_ROOM_ID,
};
use http::StatusCode;
use nanoid::nanoid;
use serde::Serialize;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::error::Result;
use crate::routes::auth::fetch_auth_state;
use crate::{Error, ServerState};

use super::util::{Auth2, HeaderReason};

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
    auth: Auth2,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let d = s.data();
    let invite = d.invite_select(code.clone()).await?;
    let (has_perm, id_target) = match invite.invite.target {
        InviteTarget::Room { room, channel } => (
            s.services()
                .perms
                .for_room(auth.user.id, room.id)
                .await?
                .has(Permission::InviteManage),
            InviteTargetId::Room {
                room_id: room.id,
                channel_id: channel.map(|t| t.id),
            },
        ),
        InviteTarget::Gdm { channel } => (
            s.services()
                .perms
                .for_channel(auth.user.id, channel.id)
                .await?
                .has(Permission::InviteManage),
            InviteTargetId::Gdm {
                channel_id: channel.id,
            },
        ),
        InviteTarget::Server => (
            s.services()
                .perms
                .for_room(auth.user.id, SERVER_ROOM_ID)
                .await?
                .has(Permission::InviteManage),
            InviteTargetId::Server,
        ),
        InviteTarget::User { user } => (false, InviteTargetId::User { user_id: user.id }),
    };
    let can_delete = auth.user.id == invite.invite.creator_id || has_perm;
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
                user_id: auth.user.id,
                session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
                reason: reason.clone(),
                ty: AuditLogEntryType::InviteDelete {
                    code: code.clone(),
                    changes: Changes::new()
                        .remove("description", &invite.invite.description)
                        .build(),
                },
            })
            .await?;
        }
        match id_target {
            InviteTargetId::Room { room_id, .. } => {
                s.broadcast_room(
                    room_id,
                    auth.user.id,
                    MessageSync::InviteDelete {
                        code,
                        target: id_target,
                    },
                )
                .await?;
            }
            InviteTargetId::Gdm { channel_id } => {
                s.broadcast_channel(
                    channel_id,
                    auth.user.id,
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
                    auth.user.id,
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
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let s = s.services();
    let invite = d.invite_select(code).await?;
    if invite.invite.creator_id == auth.user.id {
        return Ok(Json(invite).into_response());
    }
    let should_strip = match &invite.invite.target {
        InviteTarget::Room { room, .. } => {
            let perms = s.perms.for_room(auth.user.id, room.id).await?;
            !perms.has(Permission::InviteManage)
        }
        InviteTarget::Gdm { channel } => {
            let perms = s.perms.for_channel(auth.user.id, channel.id).await?;
            !perms.has(Permission::InviteManage)
        }
        InviteTarget::Server => {
            let perms = s.perms.for_room(auth.user.id, SERVER_ROOM_ID).await?;
            !perms.has(Permission::InviteManage)
        }
        InviteTarget::User { user: _ } => auth.user.id != invite.invite.creator_id,
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
    auth: Auth2,
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
            if let Ok(ban) = d.room_ban_get(room.id, auth.user.id).await {
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
            let existing = d.room_member_get(room.id, auth.user.id).await;
            if existing.is_ok_and(|e| e.membership == RoomMembership::Join) {
                return Ok(StatusCode::NO_CONTENT);
            }

            d.room_member_put(
                room.id,
                auth.user.id,
                Some(origin),
                RoomMemberPut::default(),
            )
            .await?;
            let member = d.room_member_get(room.id, auth.user.id).await?;
            srv.perms.invalidate_room(auth.user.id, room.id).await;
            srv.perms.invalidate_is_mutual(auth.user.id);
            let room_id = room.id;
            // FIXME: don't send RoomCreate to *everyone* when someone joins, just the joining user
            s.broadcast_room(
                room_id,
                auth.user.id,
                MessageSync::RoomCreate { room: room.clone() },
            )
            .await?;
            s.broadcast_room(
                room_id,
                auth.user.id,
                MessageSync::RoomMemberUpsert { member },
            )
            .await?;
        }
        InviteTarget::Gdm { channel } => {
            d.thread_member_put(channel.id, auth.user.id, Default::default())
                .await?;
            let member = d.thread_member_get(channel.id, auth.user.id).await?;
            s.broadcast_channel(
                channel.id,
                auth.user.id,
                MessageSync::ThreadMemberUpsert { member },
            )
            .await?;
        }
        InviteTarget::Server => {
            let srv = s.services();
            let user = srv.users.get(auth.user.id, None).await?;
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
                session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
                reason,
                ty: AuditLogEntryType::UserRegistered { user_id: user.id },
            })
            .await?;
            s.broadcast(MessageSync::UserUpdate { user: updated_user })?;
        }
        InviteTarget::User { user } => {
            d.user_relationship_edit(
                auth.user.id,
                user.id,
                RelationshipPatch {
                    ignore: None,
                    relation: Some(Some(RelationshipType::Friend)),
                },
            )
            .await?;
            d.user_relationship_edit(
                user.id,
                auth.user.id,
                RelationshipPatch {
                    ignore: None,
                    relation: Some(Some(RelationshipType::Friend)),
                },
            )
            .await?;

            if let Some(rel) = d.user_relationship_get(auth.user.id, user.id).await? {
                s.broadcast(MessageSync::RelationshipUpsert {
                    user_id: auth.user.id,
                    target_user_id: user.id,
                    relationship: rel,
                })?;
            }

            if let Some(rel) = d.user_relationship_get(user.id, auth.user.id).await? {
                s.broadcast(MessageSync::RelationshipUpsert {
                    user_id: user.id,
                    target_user_id: auth.user.id,
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
        InviteTarget::Gdm { .. } => return Ok(StatusCode::NO_CONTENT),
        InviteTarget::Server => SERVER_ROOM_ID,
        InviteTarget::User { .. } => return Ok(StatusCode::NO_CONTENT),
    };
    srv.rooms
        .send_welcome_message(room_id, auth.user.id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
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
    auth: Auth2,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<InviteCreate>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let d = s.data();
    let perms = s.services.perms.for_room(auth.user.id, room_id).await?;
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
        auth.user.id,
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
        user_id: auth.user.id,
        session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
        reason: reason.clone(),
        ty: AuditLogEntryType::InviteCreate { changes },
    })
    .await?;

    s.broadcast_room(
        room_id,
        auth.user.id,
        MessageSync::InviteCreate {
            invite: Box::new(invite.clone()),
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
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let perms = s.services.perms.for_room(auth.user.id, room_id).await?;

    let res = d.invite_list_room(room_id, paginate).await?;
    let items: Vec<_> = res
        .items
        .into_iter()
        .map(|i| {
            if i.invite.creator_id != auth.user.id && !perms.has(Permission::InviteManage) {
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

/// Invite channel create
///
/// Create an invite that goes to a channel
#[utoipa::path(
    post,
    path = "/channel/{channel_id}/invite",
    params(
        ("channel_id", description = "Channel id"),
    ),
    tags = ["invite", "badge.perm-opt.InviteCreate"],
    responses(
        (status = OK, body = Invite, description = "success"),
    )
)]
async fn invite_channel_create(
    Path(channel_id): Path<ChannelId>,
    auth: Auth2,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<InviteCreate>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let d = s.data();
    let channel = d.channel_get(channel_id).await?;

    let room_id = if channel.ty == ChannelType::Gdm {
        // anyone can create invites for a gdm
        None
    } else if let Some(room_id) = channel.room_id {
        let perms = s.services.perms.for_room(auth.user.id, room_id).await?;
        perms.ensure(Permission::InviteCreate)?;
        Some(room_id)
    } else {
        return Err(Error::BadStatic("Channel is not in a room or a GDM"));
    };

    let alphabet: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
        .chars()
        .collect();
    let code = InviteCode(nanoid!(8, &alphabet));
    d.invite_insert_channel(
        channel_id,
        auth.user.id,
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

    if let Some(room_id) = room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth.user.id,
            session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
            reason: reason.clone(),
            ty: AuditLogEntryType::InviteCreate { changes },
        })
        .await?;
    }

    s.broadcast_channel(
        channel_id,
        auth.user.id,
        MessageSync::InviteCreate {
            invite: Box::new(invite.clone()),
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(invite)))
}

/// Invite channel list
///
/// List invites that go to a channel
#[utoipa::path(
    get,
    path = "/channel/{channel_id}/invite",
    params(
        PaginationQuery<InviteCode>,
        ("channel_id", description = "Channel id"),
    ),
    tags = ["invite"],
    responses(
        (status = OK, body = PaginationResponse<Invite>, description = "success"),
    )
)]
async fn invite_channel_list(
    Path(channel_id): Path<ChannelId>,
    Query(paginate): Query<PaginationQuery<InviteCode>>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let channel = d.channel_get(channel_id).await?;

    let has_perm = if channel.ty == ChannelType::Gdm {
        true
    } else if let Some(room_id) = channel.room_id {
        s.services
            .perms
            .for_room(auth.user.id, room_id)
            .await?
            .has(Permission::InviteManage)
    } else {
        false
    };

    let res = d.invite_list_channel(channel_id, paginate).await?;
    let items: Vec<_> = res
        .items
        .into_iter()
        .map(|i| {
            if i.invite.creator_id != auth.user.id && !has_perm {
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
    auth: Auth2,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
    Json(patch): Json<InvitePatch>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let start_invite = d.invite_select(code.clone()).await?;

    let (has_perm, _id_target) = match start_invite.invite.target {
        InviteTarget::Room { room, channel } => (
            s.services()
                .perms
                .for_room(auth.user.id, room.id)
                .await?
                .has(Permission::InviteManage),
            InviteTargetId::Room {
                room_id: room.id,
                channel_id: channel.map(|t| t.id),
            },
        ),
        InviteTarget::Gdm { channel } => (
            s.services()
                .perms
                .for_channel(auth.user.id, channel.id)
                .await?
                .has(Permission::InviteManage),
            InviteTargetId::Gdm {
                channel_id: channel.id,
            },
        ),
        InviteTarget::Server => (
            s.services()
                .perms
                .for_room(auth.user.id, SERVER_ROOM_ID)
                .await?
                .has(Permission::InviteManage),
            InviteTargetId::Server,
        ),
        InviteTarget::User { user: _ } => (
            auth.user.id == start_invite.invite.creator_id,
            InviteTargetId::User {
                user_id: auth.user.id,
            },
        ),
    };

    let can_patch = auth.user.id == start_invite.invite.creator_id || has_perm;
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
            user_id: auth.user.id,
            session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
            reason,
            ty: AuditLogEntryType::InviteUpdate { changes },
        })
        .await?;
    }

    s.broadcast(MessageSync::InviteUpdate {
        invite: Box::new(updated_invite.clone()),
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
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<InviteCreate>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let d = s.data();
    let srv = s.services();
    let user = srv.users.get(auth.user.id, None).await?;
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
        session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
        reason: reason.clone(),
        ty: AuditLogEntryType::InviteCreate { changes },
    })
    .await?;

    s.broadcast_room(
        SERVER_ROOM_ID,
        user.id,
        MessageSync::InviteCreate {
            invite: Box::new(invite.clone()),
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
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let d = s.data();
    let user = srv.users.get(auth.user.id, None).await?;
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
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<InviteCreate>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };

    if auth.user.id != target_user_id {
        return Err(Error::NotFound);
    }

    let d = s.data();

    let alphabet: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
        .chars()
        .collect();
    let code = InviteCode(nanoid!(8, &alphabet));
    d.invite_insert_user(
        auth.user.id,
        auth.user.id,
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
        room_id: auth.user.id.into_inner().into(),
        user_id: auth.user.id,
        session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
        reason: reason.clone(),
        ty: AuditLogEntryType::InviteCreate { changes },
    })
    .await?;

    s.broadcast(MessageSync::InviteCreate {
        invite: Box::new(invite.clone()),
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
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };

    if auth.user.id != target_user_id {
        return Err(Error::NotFound);
    }

    let d = s.data();
    let res = d.invite_list_user(target_user_id, paginate).await?;

    let items: Vec<_> = res
        .items
        .into_iter()
        .map(|i| {
            if i.invite.creator_id != auth.user.id {
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

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(invite_delete))
        .routes(routes!(invite_resolve))
        .routes(routes!(invite_patch))
        .routes(routes!(invite_use))
        .routes(routes!(invite_room_create))
        .routes(routes!(invite_room_list))
        .routes(routes!(invite_channel_create))
        .routes(routes!(invite_channel_list))
        .routes(routes!(invite_server_create))
        .routes(routes!(invite_server_list))
        .routes(routes!(invite_user_create))
        .routes(routes!(invite_user_list))
}
