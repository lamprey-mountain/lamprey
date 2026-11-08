use common::v1::types::{
    AuditLogEntryType, ChannelId, ChannelType, Mentions, MentionsUser, MessageList, MessageMember,
    MessageSync, MessageType, Permission, RelationshipType, SERVER_ROOM_ID, oauth::Scope,
    thread::ThreadList,
};
use lamprey_backend_services::compat::{consts::MAX_GDM_MEMBERS, types::DbMessageCreate};

use crate::{prelude::*, util::request::ReqAuth};

#[handler(routes::thread_member_list)]
pub async fn member_list(
    req: Req<routes::thread_member_list::Endpoint>,
) -> Result<routes::thread_member_list::Response> {
    let identity = req.identity();
    identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    srv.perms
        .for_channel3(identity.user_id(), body.thread_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .check()?;

    let mut data = req.globals().begin_read().await.cast_internal()?;
    let members = data
        .thread_member_list(body.thread_id, body.pagination.clone())
        .await
        .cast_internal()?;

    Ok(routes::thread_member_list::Response { members })
}

#[handler(routes::thread_member_get)]
pub async fn member_get(
    req: Req<routes::thread_member_get::Endpoint>,
) -> Result<routes::thread_member_get::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();
    let target_user_id = body.user_id.clone().local_unwrap_or(user.id)?;

    srv.perms
        .for_channel3(Some(user.id), body.thread_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .check()?;

    let mut data = req.globals().begin_read().await.cast_internal()?;
    let member = data
        .thread_member_get(body.thread_id, target_user_id)
        .await
        .cast_internal()?;

    Ok(routes::thread_member_get::Response { member })
}

#[handler(routes::thread_member_add)]
pub async fn member_add(
    req: Req<routes::thread_member_add::Endpoint>,
) -> Result<routes::thread_member_add::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();
    let target_user_id = body.user_id.clone().local_unwrap_or(user.id)?;

    let mut perms = srv
        .perms
        .for_channel3(Some(user.id), body.thread_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    let thread = srv
        .channels
        .get(body.thread_id, Some(user.id))
        .await
        .cast_internal()?;
    thread.ensure_unarchived()?;
    thread.ensure_unremoved()?;
    if target_user_id != user.id && !thread.invitable {
        perms.needs(Permission::MemberKick);
    }
    if !thread.ty.has_members() {
        return Err(ApiError::from_code(ErrorCode::CannotEditThreadMemberList).into());
    }
    perms.needs_unlocked();
    perms.check()?;

    if thread.ty == ChannelType::Gdm {
        let mut data = globals.begin_read().await.cast_internal()?;
        let is_joining = data
            .thread_member_get(body.thread_id, target_user_id)
            .await
            .is_err();

        if is_joining {
            let count = data
                .thread_member_list_all(body.thread_id)
                .await
                .cast_internal()?
                .len() as u32;
            if count >= MAX_GDM_MEMBERS {
                return Err(ApiError::from_code(ErrorCode::GdmTooManyMembers).into());
            }

            let relationship = data
                .user_relationship_get(user.id, target_user_id)
                .await
                .cast_internal()?;
            let are_friends =
                relationship.is_some_and(|r| r.relation == Some(RelationshipType::Friend));
            if !are_friends {
                return Err(ApiError::from_code(ErrorCode::GdmRequiresFriend).into());
            }
        }
    }

    let mut data = globals.begin().await.cast_internal()?;
    data.thread_member_put(body.thread_id, target_user_id, body.member.clone())
        .await
        .cast_internal()?;
    let member = data
        .thread_member_get(body.thread_id, target_user_id)
        .await
        .cast_internal()?;

    if target_user_id != user.id {
        let message_id = data
            .message_create(DbMessageCreate {
                id: None,
                channel_id: body.thread_id,
                attachments: vec![],
                author_id: user.id,
                embeds: vec![],
                components: vec![],
                message_type: MessageType::MemberAdd(MessageMember { target_user_id }).into(),
                created_at: None,
                removed_at: None,
                flume: None,
                mentions: Mentions {
                    users: vec![MentionsUser {
                        id: target_user_id,
                        resolved_name: "(this should be ignored)".to_owned(),
                    }],
                    ..Default::default()
                },
                interaction: None,
                ephemeral: false,
            })
            .await
            .cast_internal()?;
        let message = data
            .message_get(body.thread_id, message_id)
            .await
            .cast_internal()?;
        data.commit().await.cast_internal()?;

        srv.channels.invalidate(body.thread_id).await;
        globals
            .messaging()
            .broadcast_channel(body.thread_id, MessageSync::MessageCreate { message })
            .await
            .cast_internal()?;

        if let Some(room_id) = thread.room_id {
            req.audit_log()
                .push(
                    room_id,
                    AuditLogEntryType::ThreadMemberAdd {
                        thread_id: body.thread_id,
                        user_id: target_user_id,
                    },
                )
                .success();
        }
    } else {
        data.commit().await.cast_internal()?;
    }

    globals
        .messaging()
        .broadcast_channel(
            body.thread_id,
            MessageSync::ThreadMemberUpsert {
                room_id: thread.room_id,
                thread_id: body.thread_id,
                added: vec![member.clone()],
                removed: vec![],
            },
        )
        .await
        .cast_internal()?;

    Ok(routes::thread_member_add::Response { member })
}

#[handler(routes::thread_member_delete)]
pub async fn member_delete(
    req: Req<routes::thread_member_delete::Endpoint>,
) -> Result<routes::thread_member_delete::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();
    let target_user_id = body.user_id.clone().local_unwrap_or(user.id)?;

    let mut perms = srv
        .perms
        .for_channel3(Some(user.id), body.thread_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    let thread = srv
        .channels
        .get(body.thread_id, Some(user.id))
        .await
        .cast_internal()?;
    thread.ensure_unarchived()?;
    thread.ensure_unremoved()?;
    if target_user_id != user.id {
        perms.needs(Permission::MemberKick);
    }
    if !thread.ty.has_members() {
        return Err(ApiError::from_code(ErrorCode::CannotEditThreadMemberList).into());
    }
    perms.needs_unlocked();
    perms.check()?;

    let mut data = globals.begin().await.cast_internal()?;
    data.thread_member_leave(body.thread_id, target_user_id)
        .await
        .cast_internal()?;

    if target_user_id != user.id {
        let message_id = data
            .message_create(DbMessageCreate {
                id: None,
                channel_id: body.thread_id,
                attachments: vec![],
                author_id: user.id,
                embeds: vec![],
                components: vec![],
                message_type: MessageType::MemberRemove(MessageMember { target_user_id }).into(),
                created_at: None,
                removed_at: None,
                flume: None,
                mentions: Default::default(),
                interaction: None,
                ephemeral: false,
            })
            .await
            .cast_internal()?;
        let message = data
            .message_get(body.thread_id, message_id)
            .await
            .cast_internal()?;
        data.commit().await.cast_internal()?;

        srv.channels.invalidate(body.thread_id).await;
        globals
            .messaging()
            .broadcast_channel(body.thread_id, MessageSync::MessageCreate { message })
            .await
            .cast_internal()?;

        if let Some(room_id) = thread.room_id {
            req.audit_log()
                .push(
                    room_id,
                    AuditLogEntryType::ThreadMemberRemove {
                        thread_id: body.thread_id,
                        user_id: target_user_id,
                    },
                )
                .success();
        }
    } else {
        data.commit().await.cast_internal()?;
    }

    srv.perms
        .invalidate_thread(target_user_id, body.thread_id)
        .await;

    globals
        .messaging()
        .broadcast_channel(
            body.thread_id,
            MessageSync::ThreadMemberUpsert {
                room_id: thread.room_id,
                thread_id: body.thread_id,
                added: vec![],
                removed: vec![target_user_id],
            },
        )
        .await
        .cast_internal()?;

    Ok(routes::thread_member_delete::Response {})
}

#[handler(routes::thread_list)]
pub async fn list(
    req: Req<routes::thread_list::Endpoint>,
) -> Result<routes::thread_list::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    let perms = srv
        .perms
        .for_channel3(Some(user.id), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?;

    let include_all = perms.has(Permission::ThreadManage);
    let mut data = req.globals().begin_read().await.cast_internal()?;
    let res = data
        .thread_list_active(
            user.id,
            body.pagination.clone(),
            body.channel_id,
            include_all,
        )
        .await
        .cast_internal()?;

    let channel_ids: Vec<ChannelId> = res.items.iter().map(|c| c.id).collect();
    let threads = srv
        .channels
        .get_many(&channel_ids, Some(user.id))
        .await
        .cast_internal()?;

    Ok(routes::thread_list::Response {
        body: ThreadList {
            threads,
            total: res.total,
            cursor: res.cursor,

            // TODO: impl
            room_members: vec![],
            users: vec![],
        },
    })
}

#[handler(routes::thread_list_archived)]
pub async fn list_archived(
    req: Req<routes::thread_list_archived::Endpoint>,
) -> Result<routes::thread_list_archived::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    let perms = srv
        .perms
        .for_channel3(Some(user.id), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?;

    let include_all = perms.has(Permission::ThreadManage);
    let mut data = req.globals().begin_read().await.cast_internal()?;
    let res = data
        .thread_list_archived(
            user.id,
            body.pagination.clone(),
            body.channel_id,
            include_all,
        )
        .await
        .cast_internal()?;

    let channel_ids: Vec<ChannelId> = res.items.iter().map(|c| c.id).collect();
    let threads = srv
        .channels
        .get_many(&channel_ids, Some(user.id))
        .await
        .cast_internal()?;

    Ok(routes::thread_list_archived::Response {
        body: ThreadList {
            threads,
            total: res.total,
            cursor: res.cursor,

            // TODO: impl
            room_members: vec![],
            users: vec![],
        },
    })
}

#[handler(routes::thread_list_removed)]
pub async fn list_removed(
    req: Req<routes::thread_list_removed::Endpoint>,
) -> Result<routes::thread_list_removed::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    let mut perms = srv
        .perms
        .for_channel3(Some(user.id), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    perms.needs(Permission::ThreadManage);
    perms.check()?;

    let mut data = req.globals().begin_read().await.cast_internal()?;
    let res = data
        .thread_list_removed(user.id, body.pagination.clone(), body.channel_id, true)
        .await
        .cast_internal()?;

    let channel_ids: Vec<ChannelId> = res.items.iter().map(|c| c.id).collect();
    let threads = srv
        .channels
        .get_many(&channel_ids, Some(user.id))
        .await
        .cast_internal()?;

    Ok(routes::thread_list_removed::Response {
        body: ThreadList {
            threads,
            total: res.total,
            cursor: res.cursor,

            // TODO: impl
            room_members: vec![],
            users: vec![],
        },
    })
}

#[handler(routes::thread_list_atom)]
pub async fn list_atom(
    req: Req<routes::thread_list_atom::Endpoint>,
) -> Result<routes::thread_list_atom::Response> {
    req.identity().ensure_scopes(&[Scope::Full])?;
    Err(Error::Unimplemented)
}

#[handler(routes::thread_create)]
pub async fn create(
    req: Req<routes::thread_create::Endpoint>,
) -> Result<routes::thread_create::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    if !matches!(
        body.thread.ty,
        ChannelType::ThreadPublic
            | ChannelType::ThreadPrivate
            | ChannelType::ThreadForum2
            | ChannelType::Document
    ) {
        return Err(ApiError::from_code(ErrorCode::InvalidThreadType).into());
    }

    let parent_channel = srv
        .channels
        .get(body.channel_id, Some(user.id))
        .await
        .cast_internal()?;
    let room_id = parent_channel.room_id;

    let mut thread = body.thread.clone();
    thread.parent_id = Some(body.channel_id);
    thread
        .validate()
        .map_err(|e| ApiError::with_message(ErrorCode::InvalidData, e.to_string()))?;

    let mut auth = ReqAuth::new(&req);
    let thread = srv
        .channels
        .create(&mut auth, room_id, thread, None)
        .await
        .cast_internal()?;

    Ok(routes::thread_create::Response { thread })
}

#[handler(routes::thread_create_from_message)]
pub async fn create_from_message(
    req: Req<routes::thread_create_from_message::Endpoint>,
) -> Result<routes::thread_create_from_message::Response> {
    let identity = req.identity();
    identity.ensure_user()?.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    let mut auth = ReqAuth::new(&req);
    let thread = srv
        .channels
        .create_thread_from_message(
            &mut auth,
            body.channel_id,
            body.message_id,
            body.thread.clone(),
        )
        .await
        .cast_internal()?;

    Ok(routes::thread_create_from_message::Response { thread })
}

#[handler(routes::thread_list_room)]
pub async fn list_room(
    req: Req<routes::thread_list_room::Endpoint>,
) -> Result<routes::thread_list_room::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    let snapshot = srv
        .cache
        .load_room(body.room_id, false)
        .await
        .cast_internal()?;
    let room = snapshot.get_data().ok_or(Error::Unavailable)?;

    let mut filtered_thread_ids = vec![];
    for thread in room
        .threads
        .as_ref()
        .map(|t| t.values())
        .into_iter()
        .flatten()
    {
        let thread_channel = &thread.thread;
        let thread_id = thread_channel.id;

        let perms = srv
            .perms
            .for_channel3(Some(user.id), thread_id)
            .await
            .cast_internal()?;
        let can_view = if thread_channel.ty == ChannelType::ThreadPublic {
            perms.visible
        } else {
            perms.has(Permission::ThreadManage) || thread.members.contains_key(&user.id)
        };

        if can_view {
            filtered_thread_ids.push(thread_id);
        }
    }

    let threads = srv
        .channels
        .get_many(&filtered_thread_ids, Some(user.id))
        .await
        .cast_internal()?;
    let total = threads.len() as u64;

    Ok(routes::thread_list_room::Response {
        content: ThreadList {
            threads,
            total,
            cursor: None,

            // TODO: impl
            room_members: vec![],
            users: vec![],
        },
    })
}

#[handler(routes::thread_activity)]
pub async fn activity(
    req: Req<routes::thread_activity::Endpoint>,
) -> Result<routes::thread_activity::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    srv.perms
        .for_channel3(Some(user.id), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .check()?;

    let messages = srv
        .messages
        .list_activity(body.channel_id, user.id, body.pagination.clone())
        .await
        .cast_internal()?;

    Ok(routes::thread_activity::Response {
        body: MessageList {
            messages: messages.items,

            // TODO: impl
            users: vec![],
            room_members: vec![],
            thread_members: vec![],
        },
    })
}

#[handler(routes::channel_member_search)]
pub async fn channel_member_search(
    req: Req<routes::channel_member_search::Endpoint>,
) -> Result<routes::channel_member_search::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    let mut perms = srv
        .perms
        .for_channel3(Some(user.id), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?;

    let chan = srv
        .channels
        .get(body.channel_id, None)
        .await
        .cast_internal()?;
    if chan.room_id == Some(SERVER_ROOM_ID) {
        perms.needs(Permission::ServerOversee);
    }
    perms.check()?;

    let _limit = body.search.limit.unwrap_or(10).min(100);

    Err(Error::Unimplemented)
}

export_routes!(
    member_list,
    member_get,
    member_add,
    member_delete,
    list,
    list_archived,
    list_removed,
    list_atom,
    create,
    create_from_message,
    list_room,
    activity,
    channel_member_search,
);
