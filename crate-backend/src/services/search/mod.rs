use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use common::v1::types::{
    search::{ChannelSearchRequest, MessageSearch, MessageSearchRequest},
    Channel, ChannelId, MessageId, PaginationQuery, PaginationResponse, UserId,
};
use common::v1::types::{MessageType, RoomId};
use common::v2::types::message::Message;
use futures::stream::{FuturesUnordered, StreamExt};

use crate::Error;
use crate::{error::Result, services::search::index::TantivyHandle, ServerStateInner};

mod directory;
mod index;
mod schema;
mod tokenizer;

pub use index::IndexerCommand;

pub struct ServiceSearch {
    state: Arc<ServerStateInner>,
    tantivy: TantivyHandle,
}

impl ServiceSearch {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        let tantivy = index::spawn_indexer(Arc::clone(&state));
        Self { state, tantivy }
    }

    pub async fn search_messages2(
        &self,
        auth_user_id: UserId,
        req: MessageSearchRequest,
    ) -> Result<MessageSearch> {
        let data = self.state.data();
        let srv = self.state.services();

        let vis = srv.channels.list_user_room_channels(auth_user_id).await?;
        // let visible_channel_ids: Vec<ChannelId> = vis.iter().map(|(id, _)| *id).collect();

        let offset = req.offset;

        let searcher = self.tantivy.searcher();
        let req_clone = req.clone();
        let vis_clone = vis.clone();

        let raw_result = tokio::task::spawn_blocking(move || {
            searcher.search_messages(req_clone, &vis_clone)
        })
        .await
        .map_err(|e| Error::Internal(format!("Search task failed: {}", e)))??;

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
                    .get_many(channel_id, auth_user_id, &ids)
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
                    .get_many(channel_id, auth_user_id, &reply_ids)
                    .await?;
                msgs.extend(replies);
                srv2.messages
                    .populate_reactions(channel_id, auth_user_id, &mut msgs)
                    .await?;
                srv2.messages
                    .populate_mentions(channel_id, auth_user_id, &mut msgs)
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
        // TODO: batch fetch, only fetch archived threads
        let channel_ids: HashSet<ChannelId> = messages.iter().map(|m| m.channel_id).collect();
        let mut channel_room_map: HashMap<ChannelId, Option<RoomId>> = HashMap::new();
        for channel_id in channel_ids {
            let chan = srv.channels.get(channel_id, Some(auth_user_id)).await?;
            channel_room_map.insert(channel_id, chan.room_id);
            if chan.ty.is_thread() && chan.archived_at.is_some() {
                threads.push(chan);
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
            if let Ok(cached_room) = srv.cache.load_room(room_id).await {
                for user_id in user_ids {
                    if let Some(member) = cached_room.members.get(&user_id) {
                        room_members.push(member.clone());
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
            approximate_total: raw_result.total,
        })
    }

    // TODO: deprecate
    pub async fn search_messages(
        &self,
        user_id: UserId,
        json: MessageSearchRequest,
        q: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>> {
        let data = self.state.data();
        let srv = self.state.services();
        let vis = srv.channels.list_user_room_channels(user_id).await?;
        let mut res = data.search_message(user_id, json, q, &vis).await?;

        // group messages by channel id
        let mut channel_message_indices: HashMap<ChannelId, Vec<usize>> = HashMap::new();
        for (i, message) in res.items.iter().enumerate() {
            channel_message_indices
                .entry(message.channel_id)
                .or_default()
                .push(i);
        }

        // TODO: avoid cloning
        // populate reactions
        for (channel_id, indices) in channel_message_indices {
            let mut temp_messages: Vec<Message> =
                indices.iter().map(|&i| res.items[i].clone()).collect();

            srv.messages
                .populate_reactions(channel_id, user_id, &mut temp_messages)
                .await?;

            for (i, original_index) in indices.iter().enumerate() {
                res.items[*original_index].reactions = temp_messages[i].reactions.clone();
            }
        }

        srv.messages
            .populate_threads(user_id, &mut res.items)
            .await?;

        for message in &mut res.items {
            self.state.presign_message(message).await?;
        }

        Ok(res)
    }

    pub async fn search_channels(
        &self,
        user_id: UserId,
        json: ChannelSearchRequest,
        q: PaginationQuery<ChannelId>,
    ) -> Result<PaginationResponse<Channel>> {
        let data = self.state.data();
        let srv = self.state.services();
        let vis = srv.channels.list_user_room_channels(user_id).await?;
        let res = data.search_channel(user_id, json, q, &vis).await?;
        Ok(res)
    }

    pub fn send_indexer_command(&self, cmd: IndexerCommand) -> Result<()> {
        self.tantivy
            .command_tx
            .send(cmd)
            .map_err(|e| Error::Internal(format!("Failed to send reindex command: {}", e)))?;
        Ok(())
    }
}
