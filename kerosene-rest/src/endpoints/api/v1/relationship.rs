use common::v1::types::{
    AuditLogEntryType, MessageSync, Permission, RelationshipPatch, RelationshipType, RoomList,
    SERVER_ROOM_ID,
    error::{ApiError, ErrorCode},
    util::Time,
};

use crate::prelude::*;

#[handler(routes::friend_list)]
pub async fn friend_list(
    req: Req<routes::friend_list::Endpoint>,
) -> Result<routes::friend_list::Response> {
    req.identity()
        .ensure_scopes(&[common::v1::types::oauth::Scope::Full])?;
    let auth_user = req.identity().ensure_user()?;
    let body = req.inner();
    let target_user_id = body.user_id.clone().local_unwrap_or(auth_user.id)?;

    let mut data = req.globals().begin_read().await.cast_internal()?;
    let mut friends = data
        .user_relationship_list_friends(target_user_id, body.pagination.clone())
        .await
        .cast_internal()?;

    for friend in &mut friends.items {
        friend.inner = data
            .user_relationship_get(auth_user.id, friend.user_id)
            .await
            .cast_internal()?
            .unwrap_or_default();
    }

    Ok(routes::friend_list::Response { friends })
}

#[handler(routes::friend_list_pending)]
pub async fn friend_list_pending(
    req: Req<routes::friend_list_pending::Endpoint>,
) -> Result<routes::friend_list_pending::Response> {
    req.identity()
        .ensure_scopes(&[common::v1::types::oauth::Scope::Full])?;
    let auth_user = req.identity().ensure_user()?;
    let body = req.inner();
    let target_user_id = body.user_id.clone().local_unwrap_or(auth_user.id)?;
    if target_user_id != auth_user.id {
        return Err(ApiError::from_code(ErrorCode::MissingPermissions).into());
    }

    let mut data = req.globals().begin_read().await.cast_internal()?;
    let pending = data
        .user_relationship_list_pending(auth_user.id, body.pagination.clone())
        .await
        .cast_internal()?;

    Ok(routes::friend_list_pending::Response { pending })
}

#[handler(routes::friend_add)]
pub async fn friend_add(
    req: Req<routes::friend_add::Endpoint>,
) -> Result<routes::friend_add::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[common::v1::types::oauth::Scope::Full])?;
    let auth_user = identity.ensure_user()?;
    auth_user.ensure_unsuspended()?;

    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();
    let mut data = globals.begin().await.cast_internal()?;

    let target_user = data.user_get(body.target_id).await.cast_internal()?;
    if !target_user.can_friend() {
        return Err(ApiError::from_code(ErrorCode::CannotFriendThisUser).into());
    }

    let existing = data
        .user_relationship_get(auth_user.id, body.target_id)
        .await
        .cast_internal()?;

    let reverse = data
        .user_relationship_get(body.target_id, auth_user.id)
        .await
        .cast_internal()?;

    let al = req.audit_log();

    match (
        existing.as_ref().and_then(|r| r.relation.as_ref()),
        reverse.as_ref().and_then(|r| r.relation.as_ref()),
    ) {
        (Some(RelationshipType::Incoming), Some(RelationshipType::Outgoing)) => {
            // accept friend request
            data.user_relationship_edit(
                auth_user.id,
                body.target_id,
                RelationshipPatch {
                    ignore: None,
                    relation: Some(Some(RelationshipType::Friend)),
                },
            )
            .await
            .cast_internal()?;
            data.user_relationship_edit(
                body.target_id,
                auth_user.id,
                RelationshipPatch {
                    ignore: None,
                    relation: Some(Some(RelationshipType::Friend)),
                },
            )
            .await
            .cast_internal()?;

            al.push(
                auth_user.id.into_inner().into(),
                AuditLogEntryType::FriendAccept {
                    user_id: body.target_id,
                },
            )
            .success();
        }
        (_, Some(RelationshipType::Block)) => {
            return Err(ApiError::with_message(
                ErrorCode::InvalidData,
                "blocked by this user".to_string(),
            )
            .into());
        }
        (None, None) => {
            let mut perms = srv
                .perms
                .for_room3(Some(auth_user.id), SERVER_ROOM_ID)
                .await
                .cast_internal()?
                .ensure_view()?;
            perms.needs(Permission::FriendCreate);
            perms.check()?;

            let target_prefs = srv
                .cache
                .preferences_get(body.target_id)
                .await
                .cast_internal()?;
            let friends_prefs = &target_prefs.privacy.friends;

            if let Some(pause_until) = &friends_prefs.pause_until {
                if pause_until > &Time::now_utc() {
                    return Err(ApiError::from_code(ErrorCode::FriendRequestsPaused).into());
                }
            }

            let allowed = if friends_prefs.allow_everyone {
                true
            } else {
                let mutual_room_allowed = if friends_prefs.allow_mutual_room {
                    srv.perms
                        .allows_friend_request_from_user(auth_user.id, body.target_id)
                        .await
                        .cast_internal()?
                } else {
                    false
                };

                let mutual_friend_allowed = friends_prefs.allow_mutual_friend
                    && data
                        .user_has_mutual_friend(auth_user.id, body.target_id)
                        .await
                        .cast_internal()?;

                mutual_room_allowed || mutual_friend_allowed
            };

            if !allowed {
                return Err(ApiError::from_code(ErrorCode::InvalidData).into());
            }

            data.user_relationship_edit(
                auth_user.id,
                body.target_id,
                RelationshipPatch {
                    ignore: None,
                    relation: Some(Some(RelationshipType::Outgoing)),
                },
            )
            .await
            .cast_internal()?;
            data.user_relationship_edit(
                body.target_id,
                auth_user.id,
                RelationshipPatch {
                    ignore: None,
                    relation: Some(Some(RelationshipType::Incoming)),
                },
            )
            .await
            .cast_internal()?;

            al.push(
                auth_user.id.into_inner().into(),
                AuditLogEntryType::FriendRequest {
                    user_id: body.target_id,
                },
            )
            .success();
        }
        (Some(RelationshipType::Friend), Some(RelationshipType::Friend))
        | (Some(RelationshipType::Outgoing), Some(RelationshipType::Incoming)) => {
            return Ok(routes::friend_add::Response {});
        }
        (Some(RelationshipType::Block), _) => {
            return Err(ApiError::from_code(ErrorCode::UnblockUserFirst).into());
        }
        _ => unreachable!("invalid data in database?"),
    }

    let rel = data
        .user_relationship_get(auth_user.id, body.target_id)
        .await
        .cast_internal()?;
    let reverse_rel = data
        .user_relationship_get(body.target_id, auth_user.id)
        .await
        .cast_internal()?;

    data.commit().await.cast_internal()?;

    if let Some(rel) = rel {
        globals
            .messaging()
            .broadcast_global(MessageSync::RelationshipUpsert {
                user_id: auth_user.id,
                target_user_id: body.target_id,
                relationship: rel,
            })
            .await
            .cast_internal()?;
    }

    if let Some(rel) = reverse_rel {
        globals
            .messaging()
            .broadcast_global(MessageSync::RelationshipUpsert {
                user_id: body.target_id,
                target_user_id: auth_user.id,
                relationship: rel,
            })
            .await
            .cast_internal()?;
    }

    Ok(routes::friend_add::Response {})
}

#[handler(routes::friend_remove)]
pub async fn friend_remove(
    req: Req<routes::friend_remove::Endpoint>,
) -> Result<routes::friend_remove::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[common::v1::types::oauth::Scope::Full])?;
    let auth_user = identity.ensure_user()?;
    auth_user.ensure_unsuspended()?;

    let globals = req.globals();
    let body = req.inner();
    let mut data = globals.begin().await.cast_internal()?;

    let existing = data
        .user_relationship_get(auth_user.id, body.target_id)
        .await
        .cast_internal()?;

    let mut deleted = Vec::new();

    match existing.as_ref().and_then(|r| r.relation.as_ref()) {
        r @ Some(RelationshipType::Friend)
        | r @ Some(RelationshipType::Incoming)
        | r @ Some(RelationshipType::Outgoing) => {
            data.user_relationship_delete(auth_user.id, body.target_id)
                .await
                .cast_internal()?;
            deleted.push((auth_user.id, body.target_id));

            if r == Some(&RelationshipType::Friend) {
                req.audit_log()
                    .push(
                        auth_user.id.into_inner().into(),
                        AuditLogEntryType::FriendDelete {
                            user_id: body.target_id,
                        },
                    )
                    .success();
            }

            if let Some(r) = data
                .user_relationship_get(body.target_id, auth_user.id)
                .await
                .cast_internal()?
            {
                if matches!(
                    r.relation,
                    Some(RelationshipType::Friend)
                        | Some(RelationshipType::Incoming)
                        | Some(RelationshipType::Outgoing)
                ) {
                    data.user_relationship_delete(body.target_id, auth_user.id)
                        .await
                        .cast_internal()?;
                    deleted.push((body.target_id, auth_user.id));
                }
            }
        }
        _ => {}
    }

    data.commit().await.cast_internal()?;

    for (user_id, target_user_id) in deleted {
        globals
            .messaging()
            .broadcast_global(MessageSync::RelationshipDelete {
                user_id,
                target_user_id,
            })
            .await
            .cast_internal()?;
    }

    Ok(routes::friend_remove::Response {})
}

#[handler(routes::block_list)]
pub async fn block_list(
    req: Req<routes::block_list::Endpoint>,
) -> Result<routes::block_list::Response> {
    req.identity()
        .ensure_scopes(&[common::v1::types::oauth::Scope::Full])?;
    let auth_user = req.identity().ensure_user()?;
    let body = req.inner();
    let target_user_id = body.user_id.clone().local_unwrap_or(auth_user.id)?;
    if target_user_id != auth_user.id {
        return Err(ApiError::from_code(ErrorCode::MissingPermissions).into());
    }

    let mut data = req.globals().begin_read().await.cast_internal()?;
    let blocks = data
        .user_relationship_list_blocked(auth_user.id, body.pagination.clone())
        .await
        .cast_internal()?;

    Ok(routes::block_list::Response { blocks })
}

#[handler(routes::block_add)]
pub async fn block_add(
    req: Req<routes::block_add::Endpoint>,
) -> Result<routes::block_add::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[common::v1::types::oauth::Scope::Full])?;
    let auth_user = identity.ensure_user()?;
    auth_user.ensure_unsuspended()?;

    let globals = req.globals();
    let body = req.inner();
    let mut data = globals.begin().await.cast_internal()?;

    data.user_relationship_edit(
        auth_user.id,
        body.target_id,
        RelationshipPatch {
            ignore: None,
            relation: Some(Some(RelationshipType::Block)),
        },
    )
    .await
    .cast_internal()?;

    let reverse = data
        .user_relationship_get(body.target_id, auth_user.id)
        .await
        .cast_internal()?;

    let mut deleted_reverse = false;
    if !matches!(
        reverse.as_ref().and_then(|r| r.relation.as_ref()),
        Some(&RelationshipType::Block)
    ) && reverse.is_some()
    {
        data.user_relationship_delete(body.target_id, auth_user.id)
            .await
            .cast_internal()?;
        deleted_reverse = true;
    }

    let rel = data
        .user_relationship_get(auth_user.id, body.target_id)
        .await
        .cast_internal()?
        .unwrap();

    data.commit().await.cast_internal()?;

    if deleted_reverse {
        globals
            .messaging()
            .broadcast_global(MessageSync::RelationshipDelete {
                user_id: body.target_id,
                target_user_id: auth_user.id,
            })
            .await
            .cast_internal()?;
    }

    globals
        .messaging()
        .broadcast_global(MessageSync::RelationshipUpsert {
            user_id: auth_user.id,
            target_user_id: body.target_id,
            relationship: rel,
        })
        .await
        .cast_internal()?;

    req.audit_log()
        .push(
            auth_user.id.into_inner().into(),
            AuditLogEntryType::BlockCreate {
                user_id: body.target_id,
            },
        )
        .success();

    Ok(routes::block_add::Response {})
}

#[handler(routes::block_remove)]
pub async fn block_remove(
    req: Req<routes::block_remove::Endpoint>,
) -> Result<routes::block_remove::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[common::v1::types::oauth::Scope::Full])?;
    let auth_user = identity.ensure_user()?;
    auth_user.ensure_unsuspended()?;

    let globals = req.globals();
    let body = req.inner();
    let mut data = globals.begin().await.cast_internal()?;

    let existing = data
        .user_relationship_get(auth_user.id, body.target_id)
        .await
        .cast_internal()?;

    let removed = existing
        .as_ref()
        .is_some_and(|r| r.relation == Some(RelationshipType::Block));

    if removed {
        data.user_relationship_delete(auth_user.id, body.target_id)
            .await
            .cast_internal()?;
        data.commit().await.cast_internal()?;

        globals
            .messaging()
            .broadcast_global(MessageSync::RelationshipDelete {
                user_id: auth_user.id,
                target_user_id: body.target_id,
            })
            .await
            .cast_internal()?;

        req.audit_log()
            .push(
                auth_user.id.into_inner().into(),
                AuditLogEntryType::BlockDelete {
                    user_id: body.target_id,
                },
            )
            .success();
    }

    Ok(routes::block_remove::Response {})
}

#[handler(routes::ignore_list)]
pub async fn ignore_list(
    req: Req<routes::ignore_list::Endpoint>,
) -> Result<routes::ignore_list::Response> {
    req.identity()
        .ensure_scopes(&[common::v1::types::oauth::Scope::Full])?;
    let auth_user = req.identity().ensure_user()?;
    let body = req.inner();
    let target_user_id = body.user_id.clone().local_unwrap_or(auth_user.id)?;
    if target_user_id != auth_user.id {
        return Err(ApiError::from_code(ErrorCode::MissingPermissions).into());
    }

    let mut data = req.globals().begin_read().await.cast_internal()?;
    let ignored = data
        .user_relationship_list_ignored(auth_user.id, body.pagination.clone())
        .await
        .cast_internal()?;

    Ok(routes::ignore_list::Response { ignored })
}

#[handler(routes::ignore_add)]
pub async fn ignore_add(
    req: Req<routes::ignore_add::Endpoint>,
) -> Result<routes::ignore_add::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[common::v1::types::oauth::Scope::Full])?;
    let auth_user = identity.ensure_user()?;
    auth_user.ensure_unsuspended()?;

    let globals = req.globals();
    let body = req.inner();
    let mut data = globals.begin().await.cast_internal()?;

    data.user_relationship_edit(
        auth_user.id,
        body.target_id,
        RelationshipPatch {
            ignore: Some(Some(body.ignore.clone())),
            relation: None,
        },
    )
    .await
    .cast_internal()?;

    let rel = data
        .user_relationship_get(auth_user.id, body.target_id)
        .await
        .cast_internal()?
        .unwrap();

    data.commit().await.cast_internal()?;

    globals
        .messaging()
        .broadcast_global(MessageSync::RelationshipUpsert {
            user_id: auth_user.id,
            target_user_id: body.target_id,
            relationship: rel,
        })
        .await
        .cast_internal()?;

    req.audit_log()
        .push(
            auth_user.id.into_inner().into(),
            AuditLogEntryType::IgnoreAdd {
                user_id: body.target_id,
            },
        )
        .success();

    Ok(routes::ignore_add::Response {})
}

#[handler(routes::ignore_remove)]
pub async fn ignore_remove(
    req: Req<routes::ignore_remove::Endpoint>,
) -> Result<routes::ignore_remove::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[common::v1::types::oauth::Scope::Full])?;
    let auth_user = identity.ensure_user()?;
    auth_user.ensure_unsuspended()?;

    let globals = req.globals();
    let body = req.inner();
    let mut data = globals.begin().await.cast_internal()?;

    let existing = data
        .user_relationship_get(auth_user.id, body.target_id)
        .await
        .cast_internal()?;

    let removed = existing.as_ref().is_some_and(|r| r.ignore.is_some());

    if removed {
        data.user_relationship_edit(
            auth_user.id,
            body.target_id,
            RelationshipPatch {
                ignore: Some(None),
                relation: None,
            },
        )
        .await
        .cast_internal()?;

        let mut updated_rel = existing.unwrap();
        updated_rel.ignore = None;

        data.commit().await.cast_internal()?;

        globals
            .messaging()
            .broadcast_global(MessageSync::RelationshipUpsert {
                user_id: auth_user.id,
                target_user_id: body.target_id,
                relationship: updated_rel,
            })
            .await
            .cast_internal()?;

        req.audit_log()
            .push(
                auth_user.id.into_inner().into(),
                AuditLogEntryType::IgnoreRemove {
                    user_id: body.target_id,
                },
            )
            .success();
    }

    Ok(routes::ignore_remove::Response {})
}

#[handler(routes::user_room_list)]
pub async fn user_room_list(
    req: Req<routes::user_room_list::Endpoint>,
) -> Result<routes::user_room_list::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[common::v1::types::oauth::Scope::Rooms])?;
    let auth_user = identity.ensure_user()?;
    let auth_user_id = auth_user.id;

    let globals = req.globals();
    let srv = req.services();
    let body = req.into_inner();
    let target_user_id = body.user_id.local_unwrap_or(auth_user_id)?;

    let mut data = globals.begin_read().await.cast_internal()?;
    let rooms = if auth_user_id == target_user_id {
        data.room_list(auth_user_id, body.pagination, false)
            .await
            .cast_internal()?
    } else {
        data.room_list_mutual(auth_user_id, target_user_id, body.pagination)
            .await
            .cast_internal()?
    };

    let mut out_rooms = Vec::with_capacity(rooms.items.len());
    let mut members = Vec::with_capacity(rooms.items.len());
    for room in rooms.items {
        out_rooms.push(
            srv.rooms
                .get(room.id, Some(auth_user_id))
                .await
                .cast_internal()?,
        );
        members.push(
            data.room_member_get(room.id, auth_user_id)
                .await
                .cast_internal()?,
        );
    }

    Ok(routes::user_room_list::Response {
        room_list: RoomList {
            rooms: out_rooms,
            members,
        },
    })
}

export_routes!(
    friend_list,
    friend_list_pending,
    user_room_list,
    friend_add,
    friend_remove,
    block_list,
    block_add,
    block_remove,
    ignore_list,
    ignore_add,
    ignore_remove
);
