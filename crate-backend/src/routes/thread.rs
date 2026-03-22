use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::routes;
use common::v1::types::thread::ThreadListRoom;
use common::v1::types::{
    error::{ApiError, ErrorCode},
    AuditLogEntryType, Channel, ChannelCreate, ChannelId, ChannelMemberSearch,
    ChannelMemberSearchResponse, ChannelType, Mentions, MentionsUser, Message, MessageId,
    MessageMember, MessageSync, MessageType, RelationshipType, RoomId, ThreadMember,
    ThreadMemberPut, UserId, SERVER_ROOM_ID,
};
use http::StatusCode;
use lamprey_macros::handler;
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::types::UserIdReq;
use crate::ServerState;

use super::util::Auth;
use crate::error::{Error, Result};
use crate::routes2;

/// Thread member list
#[handler(routes::thread_member_list)]
async fn thread_member_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::thread_member_list::Request,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let perms = s
        .services()
        .perms
        .for_channel(auth.user.id, req.thread_id)
        .await?;
    perms.ensure(common::v1::types::Permission::ChannelView)?;
    let res = d.thread_member_list(req.thread_id, req.pagination).await?;
    Ok(Json(res))
}

/// Thread member get
#[handler(routes::thread_member_get)]
async fn thread_member_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::thread_member_get::Request,
) -> Result<impl IntoResponse> {
    let target_user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    let perms = s
        .services()
        .perms
        .for_channel(auth.user.id, req.thread_id)
        .await?;
    perms.ensure(common::v1::types::Permission::ChannelView)?;
    let res = d.thread_member_get(req.thread_id, target_user_id).await?;
    Ok(Json(res))
}

/// Thread member add
#[handler(routes::thread_member_add)]
async fn thread_member_add(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::thread_member_add::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    // ThreadMemberPut is empty, no validation needed
    let target_user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, req.thread_id).await?;
    perms.ensure(common::v1::types::Permission::ChannelView)?;
    let thread = srv.channels.get(req.thread_id, Some(auth.user.id)).await?;
    thread.ensure_unarchived()?;
    thread.ensure_unremoved()?;
    if target_user_id != auth.user.id {
        if !thread.invitable {
            perms.ensure(common::v1::types::Permission::MemberKick)?;
        }
    }
    if !thread.ty.has_members() {
        return Err(ApiError::from_code(ErrorCode::CannotEditThreadMemberList).into());
    }
    perms.ensure_unlocked()?;

    if thread.ty == ChannelType::Gdm {
        let is_joining = d
            .thread_member_get(req.thread_id, target_user_id)
            .await
            .is_err();

        if is_joining {
            let count = d.thread_member_list_all(req.thread_id).await?.len() as u32;
            if count >= crate::consts::MAX_GDM_MEMBERS {
                return Err(ApiError::from_code(ErrorCode::GdmTooManyMembers).into());
            }

            let relationship = d
                .user_relationship_get(auth.user.id, target_user_id)
                .await?;

            let are_friends =
                relationship.is_some_and(|r| r.relation == Some(RelationshipType::Friend));

            if !are_friends {
                return Err(ApiError::from_code(ErrorCode::GdmRequiresFriend).into());
            }
        }
    }

    d.thread_member_put(req.thread_id, target_user_id, req.member)
        .await?;
    let res = d.thread_member_get(req.thread_id, target_user_id).await?;

    if target_user_id != auth.user.id {
        let message_id = d
            .message_create(crate::types::DbMessageCreate {
                id: None,
                channel_id: req.thread_id,
                attachment_ids: vec![],
                author_id: auth.user.id,
                embeds: vec![],
                message_type: MessageType::MemberAdd(MessageMember { target_user_id }).into(),
                created_at: None,
                removed_at: None,
                mentions: Mentions {
                    users: vec![MentionsUser {
                        id: target_user_id,
                        resolved_name: "(this should be ignored)".to_owned(),
                    }],
                    ..Default::default()
                },
            })
            .await?;
        let message = srv
            .messages
            .get(req.thread_id, message_id, auth.user.id)
            .await?;
        srv.channels.invalidate(req.thread_id).await;
        s.broadcast_channel(
            req.thread_id,
            auth.user.id,
            MessageSync::MessageCreate {
                message: message.clone(),
            },
        )
        .await?;

        if let Some(room_id) = thread.room_id {
            let al = auth.audit_log(room_id);
            al.commit_success(AuditLogEntryType::ThreadMemberAdd {
                thread_id: req.thread_id,
                user_id: target_user_id,
            })
            .await?;
        }
    }

    s.broadcast_channel(
        req.thread_id,
        auth.user.id,
        MessageSync::ThreadMemberUpsert {
            room_id: thread.room_id,
            thread_id: req.thread_id,
            added: vec![res.clone()],
            removed: vec![],
        },
    )
    .await?;
    Ok(Json(res))
}

/// Thread member delete
#[handler(routes::thread_member_delete)]
async fn thread_member_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::thread_member_delete::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let target_user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, req.thread_id).await?;
    perms.ensure(common::v1::types::Permission::ChannelView)?;
    let thread = srv.channels.get(req.thread_id, Some(auth.user.id)).await?;
    thread.ensure_unarchived()?;
    thread.ensure_unremoved()?;
    if target_user_id != auth.user.id {
        perms.ensure(common::v1::types::Permission::MemberKick)?;
    }

    if !thread.ty.has_members() {
        return Err(ApiError::from_code(ErrorCode::CannotEditThreadMemberList).into());
    }
    perms.ensure_unlocked()?;

    d.thread_member_leave(req.thread_id, target_user_id).await?;

    s.services()
        .perms
        .invalidate_thread(target_user_id, req.thread_id)
        .await;

    if target_user_id != auth.user.id {
        let message_id = d
            .message_create(crate::types::DbMessageCreate {
                id: None,
                channel_id: req.thread_id,
                attachment_ids: vec![],
                author_id: auth.user.id,
                embeds: vec![],
                message_type: MessageType::MemberRemove(MessageMember { target_user_id }).into(),
                created_at: None,
                removed_at: None,
                mentions: Default::default(),
            })
            .await?;
        let message = srv
            .messages
            .get(req.thread_id, message_id, auth.user.id)
            .await?;
        srv.channels.invalidate(req.thread_id).await;
        s.broadcast_channel(
            req.thread_id,
            auth.user.id,
            MessageSync::MessageCreate {
                message: message.clone(),
            },
        )
        .await?;

        if let Some(room_id) = thread.room_id {
            let al = auth.audit_log(room_id);
            al.commit_success(AuditLogEntryType::ThreadMemberRemove {
                thread_id: req.thread_id,
                user_id: target_user_id,
            })
            .await?;
        }
    }

    s.broadcast_channel(
        req.thread_id,
        auth.user.id,
        MessageSync::ThreadMemberUpsert {
            room_id: thread.room_id,
            thread_id: req.thread_id,
            added: vec![],
            removed: vec![target_user_id],
        },
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Thread list
#[handler(routes::thread_list)]
async fn thread_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::thread_list::Request,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(common::v1::types::Permission::ChannelView)?;

    let include_all = perms.has(common::v1::types::Permission::ThreadManage);
    let mut res = data
        .thread_list_active(auth.user.id, req.pagination, req.channel_id, include_all)
        .await?;

    let channel_ids: Vec<ChannelId> = res.items.iter().map(|c| c.id).collect();
    res.items = srv
        .channels
        .get_many(&channel_ids, Some(auth.user.id))
        .await?;
    Ok(Json(res))
}

/// Thread list archived
#[handler(routes::thread_list_archived)]
async fn thread_list_archived(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::thread_list_archived::Request,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(common::v1::types::Permission::ChannelView)?;

    let include_all = perms.has(common::v1::types::Permission::ThreadManage);
    let mut res = data
        .thread_list_archived(auth.user.id, req.pagination, req.channel_id, include_all)
        .await?;

    let channel_ids: Vec<ChannelId> = res.items.iter().map(|c| c.id).collect();
    res.items = srv
        .channels
        .get_many(&channel_ids, Some(auth.user.id))
        .await?;
    Ok(Json(res))
}

/// Thread list removed
#[handler(routes::thread_list_removed)]
async fn thread_list_removed(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::thread_list_removed::Request,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(common::v1::types::Permission::ThreadManage)?;

    let mut res = data
        .thread_list_removed(auth.user.id, req.pagination, req.channel_id, true)
        .await?;

    let channel_ids: Vec<ChannelId> = res.items.iter().map(|c| c.id).collect();
    res.items = srv
        .channels
        .get_many(&channel_ids, Some(auth.user.id))
        .await?;
    Ok(Json(res))
}

/// Thread list atom/rss (TODO)
#[handler(routes::thread_list_atom)]
async fn thread_list_atom(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::thread_list_atom::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Thread create
#[handler(routes::thread_create)]
async fn thread_create(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::thread_create::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    if !matches!(
        req.thread.ty,
        ChannelType::ThreadPublic
            | ChannelType::ThreadPrivate
            | ChannelType::ThreadForum2
            | ChannelType::Document
    ) {
        return Err(ApiError::from_code(ErrorCode::InvalidThreadType).into());
    }

    let parent_channel = s
        .services()
        .channels
        .get(req.channel_id, Some(auth.user.id))
        .await?;
    let room_id = parent_channel.room_id;

    let mut json = req.thread;
    if json.auto_archive_duration.is_none() {
        json.auto_archive_duration = parent_channel.default_auto_archive_duration;
    }

    json.parent_id = Some(req.channel_id);
    json.validate()?;

    let channel = s
        .services()
        .channels
        .create_channel(&auth, room_id, json, None)
        .await?;

    Ok(Json(channel))
}

/// Thread create from message
#[handler(routes::thread_create_from_message)]
async fn thread_create_from_message(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::thread_create_from_message::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let channel = s
        .services()
        .channels
        .create_thread_from_message(&auth, req.channel_id, req.message_id, req.thread)
        .await?;

    Ok(Json(channel))
}

/// Thread list room
#[handler(routes::thread_list_room)]
async fn thread_list_room(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::thread_list_room::Request,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let user_id = auth.user.id;

    let _perms = srv.perms.for_room(user_id, req.room_id).await?;
    let snapshot = srv.cache.load_room(req.room_id, false).await?;

    let mut filtered_thread_ids = vec![];

    for thread in snapshot.get_data().unwrap().threads.values() {
        let thread_channel = &thread.thread;
        let thread_id = thread_channel.id;

        let perms = srv.perms.for_channel(user_id, thread_id).await?;
        let can_view = if thread_channel.ty == ChannelType::ThreadPublic {
            perms.has(common::v1::types::Permission::ChannelView)
        } else {
            perms.has(common::v1::types::Permission::ThreadManage)
                || thread.members.contains_key(&user_id)
        };

        if can_view {
            filtered_thread_ids.push(thread_id);
        }
    }

    let threads = srv
        .channels
        .get_many(&filtered_thread_ids, Some(user_id))
        .await?;

    Ok(Json(ThreadListRoom { threads }))
}

/// Thread activity
#[handler(routes::thread_activity)]
async fn thread_activity(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::thread_activity::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let srv = s.services();

    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(common::v1::types::Permission::ChannelView)?;

    let res = srv
        .messages
        .list_activity(req.channel_id, auth.user.id, req.pagination)
        .await?;

    Ok(Json(res))
}

/// Channel member search
#[handler(routes::channel_member_search)]
async fn channel_member_search(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::channel_member_search::Request,
) -> Result<impl IntoResponse> {
    let _d = s.data();
    let srv = s.services();

    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(common::v1::types::Permission::ChannelView)?;

    let chan = srv.channels.get(req.channel_id, None).await?;

    if chan.room_id == Some(SERVER_ROOM_ID) {
        perms.ensure(common::v1::types::Permission::ServerOversee)?;
    }

    let _limit = req.search.limit.unwrap_or(10).min(100);

    Ok(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(thread_create))
        .routes(routes2!(thread_create_from_message))
        .routes(routes2!(thread_member_list))
        .routes(routes2!(thread_member_get))
        .routes(routes2!(thread_member_add))
        .routes(routes2!(thread_member_delete))
        .routes(routes2!(thread_list))
        .routes(routes2!(thread_list_archived))
        .routes(routes2!(thread_list_removed))
        .routes(routes2!(thread_list_atom))
        .routes(routes2!(thread_list_room))
        .routes(routes2!(thread_activity))
        .routes(routes2!(channel_member_search))
}
