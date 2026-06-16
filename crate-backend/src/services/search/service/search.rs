use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use common::v2::types::media::Media;
use futures::stream::{FuturesUnordered, StreamExt};
use tracing::trace;

use common::v1::types::message::MessageType;
use common::v1::types::search::{
    AuditLogSearch, AuditLogSearchRequest, ChannelSearch, ChannelSearchRequest, MediaSearch,
    MediaSearchRequest, MessageSearch, MessageSearchRequest, RoomSearch, RoomSearchRequest,
    UserSearch, UserSearchRequest,
};
use common::v1::types::{
    AuditLogEntryId, ChannelId, MediaId, MessageId, PaginationQuery, RoomId, UserId,
};

use crate::Result;
use crate::services::search::ServiceSearch;
use crate::services::search::index::searcher::{
    ContentSearcher, TantivySearchAuditLogEntries, TantivySearchChannels, TantivySearchMedia,
    TantivySearchMessages, TantivySearchRooms, TantivySearchUsers,
};
use crate::services::search::util::visibility::{
    ChannelVisibility, SearchAuditLogVisibility, SearchChannelsVisibility, SearchMediaVisibility,
    SearchMessagesVisibility, SearchRoomsVisibility,
};

impl ServiceSearch {
    pub async fn search_messages(
        &self,
        auth_user_id: UserId,
        req: MessageSearchRequest,
    ) -> Result<MessageSearch> {
        let srv = self.state.services();
        let vis = srv.channels.list_user_room_channels(auth_user_id).await?;
        trace!(count = vis.len(), "visible channels");
        let visibility = SearchMessagesVisibility::Filtered(
            vis.into_iter()
                .map(|(id, can_view_private_threads)| ChannelVisibility {
                    id,
                    can_view_private_threads,
                })
                .collect(),
        );
        self.search_messages_inner(auth_user_id, visibility, req)
            .await
    }

    pub async fn search_all_messages(
        &self,
        auth_user_id: UserId,
        req: MessageSearchRequest,
    ) -> Result<MessageSearch> {
        self.search_messages_inner(auth_user_id, SearchMessagesVisibility::Everything, req)
            .await
    }

    async fn search_messages_inner(
        &self,
        auth_user_id: UserId,
        visibility: SearchMessagesVisibility,
        req: MessageSearchRequest,
    ) -> Result<MessageSearch> {
        let mut data = self.state.data();
        let srv = self.state.services();

        let offset = req.inner.offset;

        // TODO: use instrumentation instead of trace! for "starting search task"
        trace!("starting search task");
        let index = self.get_index().await?;
        let searcher = index.searcher().await?;
        let cs = ContentSearcher::new(searcher);
        let raw_result = cs
            .search_messages(TantivySearchMessages { req, visibility })
            .await?;
        trace!("finished search task");

        // split messages by channel
        let mut channel_groups: HashMap<ChannelId, Vec<MessageId>> = HashMap::new();
        for item in &raw_result.items {
            channel_groups
                .entry(item.channel_id)
                .or_default()
                .push(item.id);
        }

        // fetch all messages and replies
        let mut group_futs = FuturesUnordered::new();
        for (channel_id, ids) in channel_groups {
            let srv2 = Arc::clone(&srv);
            group_futs.push(async move {
                let mut msgs = srv2
                    .messages
                    .get_many(channel_id, Some(auth_user_id), &ids)
                    .await?;
                let reply_ids: Vec<_> = msgs
                    .iter()
                    .filter_map(|m| match &m.latest_version.message_type {
                        MessageType::DefaultMarkdown(m) => m.reply_id,
                        _ => None,
                    })
                    .collect();
                let replies = srv2
                    .messages
                    .get_many(channel_id, Some(auth_user_id), &reply_ids)
                    .await?;
                msgs.extend(replies);
                srv2.messages
                    .populate_all(channel_id, Some(auth_user_id), &mut msgs)
                    .await?;
                Result::Ok(msgs)
            });
        }

        let mut messages = Vec::new();
        while let Some(res) = group_futs.next().await {
            messages.extend(res?);
        }

        let author_ids: HashSet<_> = messages.iter().map(|m| m.author_id).collect();

        let mut threads = Vec::new();
        let mut room_members = Vec::new();
        let mut thread_members = Vec::new();

        // fetch threads
        let channel_ids: HashSet<_> = messages.iter().map(|m| m.channel_id).collect();
        let channel_ids: Vec<_> = channel_ids.into_iter().collect();
        let mut channel_room_map: HashMap<ChannelId, Option<RoomId>> = HashMap::new();

        if !channel_ids.is_empty() {
            let channels = srv
                .channels
                .get_many(&channel_ids, Some(auth_user_id))
                .await?;
            for chan in channels {
                channel_room_map.insert(chan.id, chan.room_id);
                if chan.is_thread() && chan.is_archived() {
                    threads.push(chan);
                }
            }
        }

        // fetch room members
        let mut room_users_map: HashMap<RoomId, HashSet<UserId>> = HashMap::new();
        for msg in &messages {
            if let Some(Some(room_id)) = channel_room_map.get(&msg.channel_id) {
                room_users_map
                    .entry(*room_id)
                    .or_default()
                    .insert(msg.author_id);
            }
        }

        for (room_id, user_ids) in room_users_map {
            if let Ok(cached_room) = srv.cache.load_room(room_id, true).await {
                if let Some(data) = cached_room.get_data() {
                    for user_id in user_ids {
                        if let Some(member) = data.members.get(&user_id) {
                            room_members.push(member.member.clone());
                        }
                    }
                }
            }
        }

        // fetch thread members for the requesting user
        // FIXME: return thread members for message authors too
        let thread_ids: Vec<ChannelId> = threads.iter().map(|c| c.id).collect();
        if !thread_ids.is_empty() {
            if let Ok(members) = data
                .thread_member_bulk_fetch(auth_user_id, &thread_ids)
                .await
            {
                thread_members.extend(members.into_iter().map(|(_, m)| m));
            }
        }

        let users = data
            .user_get_many(&author_ids.into_iter().collect::<Vec<_>>())
            .await?;

        let has_more = (offset as u64 + raw_result.items.len() as u64) < raw_result.total;

        Ok(MessageSearch {
            results: raw_result.items.iter().map(|r| r.id).collect(),
            messages,
            users,
            threads,
            room_members,
            thread_members,
            has_more,
            total: raw_result.total,
            cursor: None,
        })
    }

    // TODO: redesign this?
    // pub async fn search_channels(
    //     &self,
    //     user_id: UserId,
    //     visibility: SearchChannelsVisibility,
    //     req: ChannelSearchRequest,
    // ) -> Result<ChannelSearch> {}

    pub async fn search_channels(
        &self,
        user_id: UserId,
        req: ChannelSearchRequest,
    ) -> Result<ChannelSearch> {
        let srv = self.state.services();
        let mut data = self.state.data();
        let index = self.get_index().await?;
        let searcher = index.searcher().await?;
        let cs = ContentSearcher::new(searcher);

        let rooms = data
            .room_list(
                user_id,
                PaginationQuery {
                    from: None,
                    to: None,
                    dir: None,
                    limit: Some(1024),
                },
                false,
            )
            .await?;
        let room_ids: Vec<RoomId> = rooms.items.iter().map(|r| r.id).collect();

        let raw_result = cs
            .search_channels(TantivySearchChannels {
                req: req.clone(),
                visibility: SearchChannelsVisibility::Filtered {
                    user_ids: vec![user_id],
                    room_ids,
                },
            })
            .await?;

        let channel_ids: Vec<ChannelId> = raw_result.items.iter().map(|i| i.id).collect();
        let channels = if channel_ids.is_empty() {
            vec![]
        } else {
            srv.channels.get_many(&channel_ids, Some(user_id)).await?
        };

        let has_more = (req.inner.offset as u64 + channel_ids.len() as u64) < raw_result.total;

        Ok(ChannelSearch {
            results: channel_ids,
            channels,
            has_more,
            total: raw_result.total,
            cursor: None,
        })
    }

    pub async fn search_rooms(
        &self,
        visibility: SearchRoomsVisibility,
        req: RoomSearchRequest,
    ) -> Result<RoomSearch> {
        let srv = self.state.services();
        let index = self.get_index().await?;
        let searcher = index.searcher().await?;
        let cs = ContentSearcher::new(searcher);

        let raw_result = cs
            .search_rooms(TantivySearchRooms {
                req: req.clone(),
                visibility,
            })
            .await?;

        let room_ids: Vec<RoomId> = raw_result.items.iter().map(|i| i.id).collect();
        let rooms = if room_ids.is_empty() {
            vec![]
        } else {
            srv.rooms.get_many(&room_ids, None).await?
        };

        let has_more = (req.inner.offset as u64 + room_ids.len() as u64) < raw_result.total;

        Ok(RoomSearch {
            results: room_ids,
            rooms,
            has_more,
            total: raw_result.total,
            cursor: None,
        })
    }

    pub async fn search_users(&self, req: UserSearchRequest) -> Result<UserSearch> {
        let srv = self.state.services();
        let index = self.get_index().await?;
        let searcher = index.searcher().await?;
        let cs = ContentSearcher::new(searcher);

        let raw_result = cs
            .search_users(TantivySearchUsers { req: req.clone() })
            .await?;

        let user_ids: Vec<UserId> = raw_result.items.iter().map(|i| i.id).collect();
        let users = if user_ids.is_empty() {
            vec![]
        } else {
            srv.users.get_many(&user_ids).await?
        };

        let has_more = (req.inner.offset as u64 + user_ids.len() as u64) < raw_result.total;

        Ok(UserSearch {
            results: user_ids,
            users,
            has_more,
            total: raw_result.total,
            cursor: None,
        })
    }

    pub async fn search_media(
        &self,
        visibility: SearchMediaVisibility,
        req: MediaSearchRequest,
    ) -> Result<MediaSearch> {
        let srv = self.state.services();
        let index = self.get_index().await?;
        let searcher = index.searcher().await?;
        let cs = ContentSearcher::new(searcher);

        let raw_result = cs
            .search_media(TantivySearchMedia {
                req: req.clone(),
                visibility,
            })
            .await?;

        let media_ids: Vec<MediaId> = raw_result.items.iter().map(|i| i.id).collect();
        let media = if media_ids.is_empty() {
            vec![]
        } else {
            srv.media.get_many(&media_ids).await?
        };

        let media: Vec<Media> = media.iter().map(|m| (*m.media()).clone()).collect();
        let user_ids: Vec<UserId> = media.iter().filter_map(|m| m.user_id).collect();
        let users = if user_ids.is_empty() {
            vec![]
        } else {
            srv.users.get_many(&user_ids).await?
        };

        let has_more = (req.inner.offset as u64 + media_ids.len() as u64) < raw_result.total;

        Ok(MediaSearch {
            results: media_ids,
            media,
            users,
            has_more,
            total: raw_result.total,
            cursor: None,
        })
    }

    /// search the audit log
    pub async fn search_audit_log(
        &self,
        vis: SearchAuditLogVisibility,
        req: AuditLogSearchRequest,
    ) -> Result<AuditLogSearch> {
        let index = self.get_index().await?;
        let searcher = index.searcher().await?;
        let cs = ContentSearcher::new(searcher);

        let raw_result = cs
            .search_audit_log_entries(TantivySearchAuditLogEntries {
                req: req.clone(),
                visibility: vis,
            })
            .await?;

        let results: Vec<AuditLogEntryId> = raw_result.items.iter().map(|e| e.id).collect();

        // fetch entries from database
        // PERF: batch fetch audit logs
        let mut entries = Vec::new();
        let mut data = self.state.data();
        for item in &raw_result.items {
            if let Ok(entry) = data.audit_logs_get(item.id).await {
                entries.push(entry);
            }
        }

        let has_more = (req.inner.offset as u64 + results.len() as u64) < raw_result.total;

        Ok(AuditLogSearch {
            results,
            entries,
            has_more,
            total: raw_result.total,
            cursor: None,
        })
    }
}
