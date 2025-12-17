use std::collections::HashMap;
use std::sync::Arc;

use common::v1::types::{
    search::{SearchChannelsRequest, SearchMessageRequest},
    Channel, ChannelId, Message, MessageId, PaginationQuery, PaginationResponse, UserId,
};

use crate::{error::Result, ServerStateInner};

pub struct ServiceSearch {
    state: Arc<ServerStateInner>,
}

impl ServiceSearch {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self { state }
    }

    pub async fn search_messages(
        &self,
        user_id: UserId,
        json: SearchMessageRequest,
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
        json: SearchChannelsRequest,
        q: PaginationQuery<ChannelId>,
    ) -> Result<PaginationResponse<Channel>> {
        let data = self.state.data();
        let srv = self.state.services();
        let vis = srv.channels.list_user_room_channels(user_id).await?;
        let res = data.search_channel(user_id, json, q, &vis).await?;
        Ok(res)
    }
}
