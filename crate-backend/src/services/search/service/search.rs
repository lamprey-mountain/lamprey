use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use futures::stream::{FuturesUnordered, StreamExt};
use tracing::trace;

use common::v1::types::message::MessageType;
use common::v1::types::search::{
    ChannelSearch, ChannelSearchRequest, MessageSearch, MessageSearchRequest, RoomSearch,
    RoomSearchRequest,
};
use common::v1::types::{ChannelId, MessageId, RoomId, UserId};

use crate::error::Error;
use crate::services::search::searcher::content::{SearchChannels, SearchMessages};
use crate::services::search::ServiceSearch;
use crate::Result;

impl ServiceSearch {
    pub async fn search_messages(
        &self,
        auth_user_id: UserId,
        req: MessageSearchRequest,
    ) -> Result<MessageSearch> {
        let mut data = self.state.data();
        let srv = self.state.services();

        let vis = srv.channels.list_user_room_channels(auth_user_id).await?;
        trace!(count = vis.len(), "visible channels");

        let offset = req.inner.offset;

        let req_clone = req.clone();
        let vis_clone = vis.clone();

        trace!("starting search task");
        let searcher = self.get_content_searcher().await?;
        let raw_result = tokio::task::spawn_blocking(move || {
            searcher.search_messages(SearchMessages {
                req: req_clone,
                visible_channel_ids: vis_clone,
            })
        })
        .await
        .map_err(|e| Error::Internal(format!("Search task failed: {}", e)))??;
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

    pub async fn search_channels(
        &self,
        user_id: UserId,
        req: ChannelSearchRequest,
    ) -> Result<ChannelSearch> {
        let srv = self.state.services();
        let _data = self.state.data();

        let vis = srv.channels.list_user_room_channels(user_id).await?;
        trace!(count = vis.len(), "visible channels for search");

        let visible_room_ids: Vec<RoomId> = {
            if vis.is_empty() {
                vec![]
            } else {
                let channel_ids: Vec<ChannelId> = vis.iter().map(|(id, _)| *id).collect();
                let channels = srv.channels.get_many(&channel_ids, Some(user_id)).await?;
                let mut room_ids: HashSet<RoomId> = HashSet::new();
                for chan in &channels {
                    if let Some(room_id) = chan.room_id {
                        room_ids.insert(room_id);
                    }
                }
                room_ids.into_iter().collect()
            }
        };

        if visible_room_ids.is_empty() {
            return Ok(ChannelSearch {
                results: vec![],
                channels: vec![],
                has_more: false,
                total: 0,
                cursor: None,
            });
        }

        let req_clone = req.clone();
        trace!("starting channel search task");
        let searcher = self.get_content_searcher().await?;
        let raw_result = tokio::task::spawn_blocking(move || {
            searcher.search_channels(SearchChannels {
                req: req_clone,
                visible_room_ids,
            })
        })
        .await
        .map_err(|e| Error::Internal(format!("Search task failed: {}", e)))??;
        trace!("finished channel search task");

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
        user_id: UserId,
        req: RoomSearchRequest,
    ) -> Result<RoomSearch> {
        todo!()
    }

    // TODO: search_users
    // TODO: search_media
    // TODO: search_audit_log
}
