#![allow(dead_code)] // TEMP

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use common::v1::types::search::{ChannelSearchOrderField, Order};
use common::v1::types::{
    search::{ChannelSearchRequest, MessageSearch, MessageSearchRequest},
    Channel, ChannelId, MessageId, PaginationQuery, PaginationResponse, RoomId, UserId,
};
use common::v2::types::message::MessageType;
use futures::stream::{FuturesUnordered, StreamExt};
use kameo::actor::Spawn;
use lamprey_backend_core::types::admin::SearchIndexStats;
use tokio::sync::OnceCell;
use tracing::trace;

use crate::services::search::import::ChannelReindexerManager;
use crate::services::search::index::IndexManager;
use crate::services::search::schema::content::ContentSchema;
use crate::services::search::schema::ContentIndex;
use crate::services::search::searcher::content::{ContentSearcher, SearchChannels, SearchMessages};
use crate::Error;
use crate::{error::Result, ServerStateInner};

mod directory;
mod import;
mod index;
mod schema;
mod searcher;
mod tokenizer;
mod util;

pub struct ServiceSearch {
    state: Arc<ServerStateInner>,
    index_manager: IndexManager,

    /// searcher for messages, channels, rooms, and other generic content
    content_searcher: OnceCell<Arc<ContentSearcher>>,
    // /// index for room (and server) analytics
    // room_analytics: ActorRef<IndexActor>,

    // /// index for document history
    // document_history: ActorRef<IndexActor>,
}

impl ServiceSearch {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        let index_manager = IndexManager::new(Arc::clone(&state));

        Self {
            state,
            index_manager,
            content_searcher: OnceCell::new(),
            // room_analytics,
            // document_history,
        }
    }

    async fn get_content_searcher(&self) -> Result<Arc<ContentSearcher>> {
        let server_state = Arc::clone(&self.state);

        self.content_searcher
            .get_or_try_init(|| async {
                // open or create the index
                let (writer, reader) = self.index_manager.open(ContentIndex::default()).await?;

                // begin reindexing channels
                ChannelReindexerManager::spawn((
                    server_state,
                    writer,
                    4, // TODO: finetune (maybe allow setting in config?)
                ));

                Ok(Arc::new(ContentSearcher::new(
                    reader,
                    ContentSchema::default(),
                )))
            })
            .await
            .cloned()
    }

    pub async fn search_messages(
        &self,
        auth_user_id: UserId,
        req: MessageSearchRequest,
    ) -> Result<MessageSearch> {
        let data = self.state.data();
        let srv = self.state.services();

        let vis = srv.channels.list_user_room_channels(auth_user_id).await?;
        trace!(count = vis.len(), "visible channels");

        let offset = req.offset;

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
        _q: PaginationQuery<ChannelId>,
    ) -> Result<PaginationResponse<Channel>> {
        let srv = self.state.services();
        let data = self.state.data();

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
            return Ok(PaginationResponse {
                items: vec![],
                has_more: false,
                total: 0,
                cursor: None,
            });
        }

        let is_activity_sort = req.sort_field == ChannelSearchOrderField::Activity;
        let req_clone = req.clone();

        // HACK: for activity sorting, fetch more results to re-sort in memory
        let search_req = if is_activity_sort {
            let mut modified = req.clone();
            modified.limit = req.limit.max(500);
            modified.offset = 0;
            modified
        } else {
            req.clone()
        };

        trace!("starting channel search task");
        let searcher = self.get_content_searcher().await?;
        let raw_result = tokio::task::spawn_blocking(move || {
            searcher.search_channels(SearchChannels {
                req: search_req,
                visible_room_ids,
            })
        })
        .await
        .map_err(|e| Error::Internal(format!("Search task failed: {}", e)))??;
        trace!("finished channel search task");

        let channel_ids: Vec<ChannelId> = raw_result.items.iter().map(|i| i.id).collect();
        let mut channels = if channel_ids.is_empty() {
            vec![]
        } else {
            srv.channels.get_many(&channel_ids, Some(user_id)).await?
        };

        // re-sort if needed
        if is_activity_sort {
            channels.sort_by(|a, b| {
                let a_time = a.archived_at.unwrap_or_else(|| {
                    a.last_version_id
                        .and_then(|id| id.try_into().ok())
                        .unwrap_or_else(|| a.id.try_into().unwrap())
                });
                let b_time = b.archived_at.unwrap_or_else(|| {
                    b.last_version_id
                        .and_then(|id| id.try_into().ok())
                        .unwrap_or_else(|| b.id.try_into().unwrap())
                });

                match req.sort_order {
                    Order::Ascending => a_time.cmp(&b_time),
                    Order::Descending => b_time.cmp(&a_time),
                }
            });

            // re-apply pagination after resorting
            let total = channels.len();
            let start = req.offset as usize;
            let end = (start + req.limit as usize).min(total);

            if start >= total {
                return Ok(PaginationResponse {
                    items: vec![],
                    has_more: false,
                    total: raw_result.total,
                    cursor: None,
                });
            }

            let paged_channels = channels[start..end].to_vec();
            let has_more = end < total || raw_result.total > req.limit as u64;

            return Ok(PaginationResponse {
                items: paged_channels,
                has_more,
                total: raw_result.total,
                cursor: None,
            });
        }

        let has_more = (req.offset as u64 + channel_ids.len() as u64) < raw_result.total;

        Ok(PaginationResponse {
            items: channels,
            has_more,
            total: raw_result.total,
            cursor: None,
        })
    }

    pub async fn reindex_channel(&self, channel_id: ChannelId) -> Result<()> {
        let data = self.state.data();

        if let Some(index_actor) = self.index_manager.get_index_actor("content") {
            let delete_term = index::delete_term_for_channel(channel_id);
            let _ = index_actor.tell(delete_term).await;
        }

        data.search_reindex_queue_upsert(channel_id, None).await?;
        Ok(())
    }

    pub async fn reindex_room(&self, room_id: RoomId) -> Result<()> {
        let data = self.state.data();

        if let Some(index_actor) = self.index_manager.get_index_actor("content") {
            let delete_term = index::delete_term_for_room(room_id);
            let _ = index_actor.tell(delete_term).await;
        }

        data.search_reindex_queue_upsert_room(room_id).await?;
        Ok(())
    }

    pub async fn reindex_everything(&self) -> Result<()> {
        if let Some(index_actor) = self.index_manager.get_index_actor("content") {
            let _ = index_actor.tell(index::DeleteAllDocuments).await;
        }

        let data = self.state.data();
        data.search_reindex_queue_upsert_all().await?;
        Ok(())
    }

    pub async fn get_channel_stats(&self, channel_id: ChannelId) -> Result<SearchIndexStats> {
        let data = self.state.data();
        let searcher = self.get_content_searcher().await?;

        let documents_indexed =
            tokio::task::spawn_blocking(move || searcher.count_documents_for_channel(channel_id))
                .await
                .map_err(|e| Error::Internal(format!("Search task failed: {}", e)))?
                .map_err(|e| Error::Internal(format!("Failed to count documents: {}", e)))?;

        let last_message_id = data.search_reindex_queue_get(channel_id).await?;

        Ok(SearchIndexStats {
            documents_indexed,
            last_message_id,
        })
    }
}
