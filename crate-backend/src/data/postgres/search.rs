use async_trait::async_trait;
use common::v1::types::{
    search::{SearchChannelsRequest, SearchMessageRequest},
    Channel, ChannelId, Message, MessageId, PaginationDirection, PaginationQuery,
    PaginationResponse, UserId,
};
use sqlx::{query_file_as, query_file_scalar, Acquire};
use uuid::Uuid;

use crate::{
    data::{
        postgres::{
            message::{DbMessage, DbMessageType},
            Pagination,
        },
        DataSearch,
    },
    error::Result,
    gen_paginate,
    types::{DbChannel, DbChannelType},
};

use super::Postgres;

#[async_trait]
impl DataSearch for Postgres {
    async fn search_message(
        &self,
        user_id: UserId,
        query: SearchMessageRequest,
        paginate: PaginationQuery<MessageId>,
        channel_visibility: &[(ChannelId, bool)],
    ) -> Result<PaginationResponse<Message>> {
        let p: Pagination<MessageId> = paginate.try_into()?;
        let room_ids: Vec<Uuid> = query.room_id.iter().map(|id| **id).collect();

        let mut visible_channel_ids = vec![];
        let mut manageable_channel_ids = vec![];

        for (id, can_manage) in channel_visibility {
            visible_channel_ids.push(**id);
            if *can_manage {
                manageable_channel_ids.push(**id);
            }
        }

        let channel_ids: Vec<Uuid> = query.channel_id.iter().map(|id| **id).collect();
        let user_ids: Vec<Uuid> = query.user_id.iter().map(|id| **id).collect();
        let mentions_users: Vec<Uuid> = query.mentions_users.iter().map(|id| **id).collect();
        let mentions_roles: Vec<Uuid> = query.mentions_roles.iter().map(|id| **id).collect();
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbMessage,
                "sql/search_message.sql",
                *user_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32,
                query.query,
                &room_ids,
                &channel_ids,
                &user_ids,
                query.has_attachment,
                query.has_image,
                query.has_audio,
                query.has_video,
                query.has_link,
                query.has_embed,
                query.pinned,
                &query.link_hostnames,
                &mentions_users,
                &mentions_roles,
                query.mentions_everyone,
                &visible_channel_ids,
                &manageable_channel_ids,
            ),
            query_file_scalar!(
                "sql/search_message_count.sql",
                *user_id,
                query.query,
                &room_ids,
                &channel_ids,
                &user_ids,
                query.has_attachment,
                query.has_image,
                query.has_audio,
                query.has_video,
                query.has_link,
                query.has_embed,
                query.pinned,
                &query.link_hostnames,
                &mentions_users,
                &mentions_roles,
                query.mentions_everyone,
                &visible_channel_ids,
                &manageable_channel_ids,
            ),
            |i: &Message| i.id.to_string()
        )
    }

    async fn search_channel(
        &self,
        user_id: UserId,
        query: SearchChannelsRequest,
        paginate: PaginationQuery<ChannelId>,
        channel_visibility: &[(ChannelId, bool)],
    ) -> Result<PaginationResponse<Channel>> {
        let p: Pagination<ChannelId> = paginate.try_into()?;
        let room_ids: Vec<Uuid> = query.room_id.iter().map(|id| **id).collect();
        let parent_ids: Vec<Uuid> = query.parent_id.iter().map(|id| **id).collect();

        let mut visible_channel_ids = vec![];
        let mut manageable_channel_ids = vec![];

        for (id, can_manage) in channel_visibility {
            visible_channel_ids.push(**id);
            if *can_manage {
                manageable_channel_ids.push(**id);
            }
        }

        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbChannel,
                "sql/search_channel.sql",
                *user_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32,
                query.query,
                &room_ids,
                &parent_ids,
                query.archived,
                query.removed,
                &visible_channel_ids,
                &manageable_channel_ids,
            ),
            query_file_scalar!(
                "sql/search_channel_count.sql",
                *user_id,
                query.query,
                &room_ids,
                &parent_ids,
                query.archived,
                query.removed,
                &visible_channel_ids,
                &manageable_channel_ids,
            ),
            |i: &Channel| i.id.to_string()
        )
    }
}
