use common::v1::types::components::{self, Components};
use common::v1::types::emoji::EmojiOwner;
use common::v1::types::reaction::{ReactionCount, ReactionCounts, ReactionKey, ReactionKeyParam};
use common::v2::types::MessageVerId;
use common::v2::types::media::{Media, MediaErrorReason, MediaReference};
use dashmap::DashMap;
use futures::{StreamExt, stream::FuturesUnordered};
use moka::future::Cache;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, warn};
use uuid::Uuid;

use common::v1::types::message::{Message, MessageType, MessageVersion, RepliesResponse};
use common::v1::types::misc::Color;
use common::v1::types::{
    Channel, ChannelId, ContextQuery, ContextResponse, EmbedCreate, EmbedId, Mentions,
    MentionsChannel, MentionsEmoji, MentionsRole, MentionsUser, MessageId, PaginationDirection,
    PaginationQuery, PaginationResponse, Permission, RepliesChildren, RepliesMessage, RepliesQuery,
    RoomId, SessionId, User,
};
use common::v1::types::{MediaId, UserId};
use common::v2::types::embed::{Embed, EmbedType};

use crate::prelude::*;
use crate::types::{MentionsIds, MessageWithCounts};

pub mod create;
pub mod flume;
pub mod links;
pub mod markdown;
pub mod util;

pub struct ServiceMessages {
    globals: Globals,
    pub flumes: DashMap<MessageId, flume::Flume>,
    pub idempotency_keys: Cache<(SessionId, String), Message>,
}

impl ServiceMessages {
    pub fn new(globals: Globals) -> Self {
        Self {
            globals,
            flumes: DashMap::new(),
            idempotency_keys: Cache::builder()
                .time_to_live(Duration::from_secs(300))
                .build(),
        }
    }

    /// get the first message in a channel
    pub async fn get_first(
        &self,
        thread_id: ChannelId,
        user_id: Option<UserId>,
    ) -> Result<Message> {
        let pagination = PaginationQuery {
            from: None,
            to: None,
            dir: Some(PaginationDirection::F),
            limit: Some(1),
        };

        let mut res = self
            .globals
            .begin_read()
            .await?
            .message_list(thread_id, pagination)
            .await?;

        let mut message = res.items.pop().ok_or(Error::NotFound)?;

        self.populate_all(thread_id, user_id, std::slice::from_mut(&mut message))
            .await?;

        Ok(message)
    }

    /// get the last message in a channel
    pub async fn get_last(&self, thread_id: ChannelId, user_id: Option<UserId>) -> Result<Message> {
        let pagination = PaginationQuery {
            from: None,
            to: None,
            dir: Some(PaginationDirection::B),
            limit: Some(1),
        };

        let mut res = self
            .globals
            .begin_read()
            .await?
            .message_list(thread_id, pagination)
            .await?;

        let mut message = res.items.pop().ok_or(Error::NotFound)?;

        self.populate_all(thread_id, user_id, std::slice::from_mut(&mut message))
            .await?;

        Ok(message)
    }

    pub async fn get(
        &self,
        thread_id: ChannelId,
        message_id: MessageId,
        user_id: Option<UserId>,
    ) -> Result<Message> {
        let mut message = self
            .globals
            .begin_read()
            .await?
            .message_get(thread_id, message_id)
            .await?;

        self.populate_all(thread_id, user_id, std::slice::from_mut(&mut message))
            .await?;

        Ok(message)
    }

    pub async fn get_with_counts(
        &self,
        thread_id: ChannelId,
        message_id: MessageId,
        user_id: Option<UserId>,
    ) -> Result<MessageWithCounts> {
        let mut mwc = self
            .globals
            .begin_read()
            .await?
            .message_get_with_counts(thread_id, message_id)
            .await?;

        self.populate_all(thread_id, user_id, std::slice::from_mut(&mut mwc.message))
            .await?;

        Ok(mwc)
    }

    pub async fn get_many(
        &self,
        channel_id: ChannelId,
        user_id: Option<UserId>,
        message_ids: &[MessageId],
    ) -> Result<Vec<Message>> {
        let mut messages = self
            .globals
            .begin_read()
            .await?
            .message_get_many(channel_id, message_ids)
            .await?;

        self.populate_all(channel_id, user_id, &mut messages)
            .await?;

        Ok(messages)
    }

    // pub async fn _create(
    //     &self,
    //     thread_id: ChannelId,
    //     auth: &Auth,
    //     nonce: Option<String>,
    //     json: MessageCreate,
    //     header_timestamp: Option<Time>,
    // ) -> Result<Message> {
    //     if let Some(n) = &nonce {
    //         self.idempotency_keys
    //             .try_get_with(
    //                 n.clone(),
    //                 self.create_inner(
    //                     thread_id,
    //                     auth.user.id,
    //                     Some(auth),
    //                     nonce,
    //                     json,
    //                     header_timestamp,
    //                 ),
    //             )
    //             .await
    //             .map_err(|err| err.fake_clone())
    //     } else {
    //         self.create_inner(
    //             thread_id,
    //             auth.user.id,
    //             Some(auth),
    //             nonce,
    //             json,
    //             header_timestamp,
    //         )
    //         .await
    //     }
    // }

    pub async fn fetch_full_mentions_from_ids(
        &self,
        mentions_ids: MentionsIds,
        room_id: Option<RoomId>,
    ) -> Result<Mentions> {
        let srv = self.globals.services();

        let mut mentions = Mentions {
            users: vec![],
            roles: vec![],
            channels: vec![],
            emojis: vec![],
            everyone: mentions_ids.everyone,
        };

        let users = srv.users.get_many(&mentions_ids.users).await?;
        let users_map: HashMap<UserId, User> = users.into_iter().map(|u| (u.id, u)).collect();

        if let Some(room_id) = room_id {
            let room = srv.cache.load_room(room_id, true).await?;

            for user_id in mentions_ids.users {
                let Some(user) = users_map.get(&user_id) else {
                    continue;
                };

                let resolved_name = match room.get_data() {
                    Some(d) => d
                        .members
                        .get(&user_id)
                        .and_then(|m| m.member.override_name.clone())
                        .unwrap_or_else(|| user.name.clone()),
                    None => user.name.clone(),
                };

                mentions.users.push(MentionsUser {
                    id: user_id,
                    resolved_name,
                });
            }

            for role_id in mentions_ids.roles {
                if room.get_data().unwrap().roles.contains_key(&role_id) {
                    mentions.roles.push(MentionsRole { id: role_id });
                }
            }
        } else {
            for user_id in mentions_ids.users {
                if let Some(user) = users_map.get(&user_id) {
                    mentions.users.push(MentionsUser {
                        id: user_id,
                        resolved_name: user.name.clone(),
                    });
                }
            }
        }

        let channels = srv.channels.get_many(&mentions_ids.channels, None).await?;
        for channel in channels {
            mentions.channels.push(MentionsChannel {
                id: channel.id,
                room_id: channel.room_id,
                ty: channel.ty,
                name: channel.name,
            });
        }

        let emojis = srv.cache.emoji_get_many(&mentions_ids.emojis).await?;
        for emoji in emojis {
            mentions.emojis.push(MentionsEmoji {
                id: emoji.id,
                name: emoji.name,
                animated: emoji.animated,
            });
        }

        Ok(mentions)
    }

    async fn enforce_emoji_use_external(
        &self,
        m: &Mentions,
        room_id: RoomId,
        allow: bool,
        content: &str,
    ) -> Result<String> {
        let srv = self.globals.services();
        let mut allowed_emoji = vec![];

        let emoji_ids: Vec<_> = m.emojis.iter().map(|e| e.id).collect();
        let emojis = srv.cache.emoji_get_many(&emoji_ids).await?;

        for emoji in emojis {
            let is_room_emoji = emoji.owner == Some(EmojiOwner::Room { room_id });
            if is_room_emoji || allow {
                allowed_emoji.push(emoji.id);
            }
        }

        Ok(markdown::strip_emoji(content, &allowed_emoji))
    }

    async fn fetch_mentions_data(
        &self,
        channel_id: ChannelId,
        user_id: Option<UserId>,
        messages: &[Message],
    ) -> Result<Vec<Mentions>> {
        if messages.is_empty() {
            return Ok(vec![]);
        }

        let mut data = self.globals.begin_read().await?;
        let channel = self
            .globals
            .services()
            .channels
            .get(channel_id, user_id)
            .await?;
        let room_id = channel.room_id;

        let version_ids: Vec<MessageVerId> = messages
            .iter()
            .map(|m| m.latest_version.version_id)
            .collect();
        let mentions_ids: Vec<MentionsIds> = data
            .message_fetch_mention_ids(channel_id, &version_ids)
            .await?;

        let mut mentions_list = Vec::with_capacity(messages.len());
        for mentions_ids in mentions_ids {
            let full_mentions = self
                .fetch_full_mentions_from_ids(mentions_ids, room_id)
                .await?;
            mentions_list.push(full_mentions);
        }

        Ok(mentions_list)
    }

    async fn fetch_threads_data(
        &self,
        user_id: Option<UserId>,
        messages: &[Message],
    ) -> Result<HashMap<MessageId, Channel>> {
        let mut threads_map = HashMap::new();

        let srv = self.globals.services();
        let mut thread_futs: FuturesUnordered<_> = messages
            .iter()
            .filter_map(|m| {
                let thread_id = match &m.latest_version.message_type {
                    MessageType::ThreadCreated(t) => t.thread_id,
                    _ => Some((*m.id).into()),
                };

                thread_id.map(|cid| {
                    let srv2 = Arc::clone(&srv);
                    let mid = m.id;
                    async move {
                        let thread = srv2.channels.get(cid, user_id).await;
                        (mid, thread)
                    }
                })
            })
            .collect();

        while let Some((mid, thread_result)) = thread_futs.next().await {
            if let Ok(thread) = thread_result {
                threads_map.insert(mid, thread);
            }
        }

        Ok(threads_map)
    }

    async fn fetch_reactions_data(
        &self,
        channel_id: ChannelId,
        user_id: Option<UserId>,
        messages: &[Message],
    ) -> Result<HashMap<MessageId, ReactionCounts>> {
        let mut data = self.globals.begin_read().await?;
        let message_ids: Vec<MessageId> = messages.iter().map(|m| m.id).collect();
        let reactions = data
            .reaction_fetch_all(
                channel_id,
                user_id.unwrap_or(Uuid::nil().into()),
                &message_ids,
            )
            .await?;
        let reactions_raw: HashMap<MessageId, Vec<(ReactionKeyParam, u64, bool)>> =
            reactions.into_iter().collect();

        let mut reactions_map = HashMap::new();
        for (message_id, rs) in reactions_raw {
            let mut counts = Vec::new();
            for r in &rs {
                counts.push(ReactionCount {
                    key: match &r.0 {
                        ReactionKeyParam::Text(t) => ReactionKey::Text {
                            content: t.to_owned(),
                        },
                        ReactionKeyParam::Custom(c) => {
                            let emoji = data.emoji_get(*c).await?;
                            ReactionKey::Custom(emoji)
                        }
                    },
                    count: r.1,
                    self_reacted: r.2,
                });
            }
            reactions_map.insert(message_id, ReactionCounts(counts));
        }

        Ok(reactions_map)
    }

    async fn fetch_components_data(
        &self,
        channel_id: ChannelId,
        _user_id: Option<UserId>,
        messages: &[Message],
    ) -> Result<HashMap<MessageId, Components<components::Canonical>>> {
        let mut data = self.globals.begin_read().await?;
        let version_ids: Vec<MessageVerId> = messages
            .iter()
            .map(|m| m.latest_version.version_id)
            .collect();

        let components_raw = data
            .message_fetch_components(channel_id, &version_ids)
            .await?;

        let version_to_id: HashMap<_, _> = messages
            .iter()
            .map(|m| (m.latest_version.version_id, m.id))
            .collect();

        let mut components_map = HashMap::with_capacity(components_raw.len());
        for (message_ver_id, components) in components_raw {
            let mut media_ids = vec![];
            let mut media_cache = HashMap::new();

            components.collect_media_refs(&mut media_ids);

            // PERF: fetch as a batch
            for media_id in &media_ids {
                if let Ok(media) = data.media_select(*media_id).await {
                    media_cache.insert(*media_id, media);
                }
            }

            // process components with a closure that resolves media from cache
            let components = components.into_canonical(|media_id: MediaId| {
                let media = media_cache
                    .get(&media_id)
                    .cloned()
                    .unwrap_or_else(|| {
                        warn!(message_ver_id = ?message_ver_id, media_id = ?media_id, "media not found");
                        Media::errored(media_id, (*media_id).into(), MediaErrorReason::NotFound)
                    });
                Result::Ok(media)
            });

            components_map.insert(*version_to_id.get(&message_ver_id).unwrap(), components?);
        }

        Ok(components_map)
    }

    pub async fn populate_all(
        &self,
        channel_id: ChannelId,
        user_id: Option<UserId>,
        messages: &mut [Message],
    ) -> Result<()> {
        if messages.is_empty() {
            return Ok(());
        }

        let mentions_fut = self.fetch_mentions_data(channel_id, user_id, messages);
        let threads_fut = self.fetch_threads_data(user_id, messages);
        let reactions_fut = self.fetch_reactions_data(channel_id, user_id, messages);
        let components_fut = self.fetch_components_data(channel_id, user_id, messages);

        let (mentions_data, threads_data, reactions_data, components_data) =
            tokio::try_join!(mentions_fut, threads_fut, reactions_fut, components_fut)?;

        for (i, message) in messages.iter_mut().enumerate() {
            if let Some(m) = mentions_data.get(i) {
                message.latest_version.mentions = m.clone();
            }
            if let Some(t) = threads_data.get(&message.id) {
                message.thread = Some(Box::new(t.clone()));
            }
            if let Some(r) = reactions_data.get(&message.id) {
                message.reactions = r.clone();
            }
            if let Some(c) = components_data.get(&message.id) {
                match &mut message.latest_version.message_type {
                    MessageType::DefaultMarkdown(m) => m.components = c.clone(),
                    _ => {}
                }
            }
        }

        Ok(())
    }

    pub async fn message_reply_context(
        &self,
        channel_id: ChannelId,
        root_message_id: Option<MessageId>,
        user_id: UserId,
        context: u16,
    ) -> Result<Vec<Message>> {
        let Some(start_message_id) = root_message_id else {
            return Ok(vec![]);
        };

        if context == 0 {
            return Ok(vec![]);
        }

        // Permission check
        let perms = self
            .globals
            .services()
            .perms
            .for_channel(user_id, channel_id)
            .await?;
        perms.ensure(Permission::ChannelView)?;

        let mut ancestors = self
            .globals
            .begin_read()
            .await?
            .message_get_ancestors(start_message_id, context)
            .await?;

        self.populate_all(channel_id, Some(user_id), &mut ancestors)
            .await?;

        Ok(ancestors)
    }

    async fn process_message_list(
        &self,
        channel_id: ChannelId,
        user_id: Option<UserId>,
        mut res: PaginationResponse<Message>,
    ) -> Result<PaginationResponse<Message>> {
        self.populate_all(channel_id, user_id, &mut res.items)
            .await?;

        Ok(res)
    }

    pub async fn list(
        &self,
        channel_id: ChannelId,
        user_id: Option<UserId>,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>> {
        let res = self
            .globals
            .begin_read()
            .await?
            .message_list(channel_id, pagination)
            .await?;
        self.process_message_list(channel_id, user_id, res).await
    }

    pub async fn list_deleted(
        &self,
        channel_id: ChannelId,
        user_id: Option<UserId>,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>> {
        let res = self
            .globals
            .begin_read()
            .await?
            .message_list_deleted(channel_id, pagination)
            .await?;
        self.process_message_list(channel_id, user_id, res).await
    }

    pub async fn list_removed(
        &self,
        channel_id: ChannelId,
        user_id: Option<UserId>,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>> {
        let res = self
            .globals
            .begin_read()
            .await?
            .message_list_removed(channel_id, pagination)
            .await?;
        self.process_message_list(channel_id, user_id, res).await
    }

    pub async fn list_all(
        &self,
        channel_id: ChannelId,
        user_id: Option<UserId>,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>> {
        let res = self
            .globals
            .begin_read()
            .await?
            .message_list_all(channel_id, pagination)
            .await?;
        self.process_message_list(channel_id, user_id, res).await
    }

    pub async fn list_context(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        user_id: Option<UserId>,
        query: ContextQuery,
    ) -> Result<ContextResponse> {
        let s = &self.globals;
        let mut data = s.begin_read().await?;

        let limit = query.limit.unwrap_or(10);
        if limit > 1024 {
            return Err(Error::BadStatic("limit too big"));
        }

        let before_q = PaginationQuery {
            from: Some(message_id),
            to: query.to_start,
            dir: Some(PaginationDirection::B),
            limit: Some(limit),
        };

        let after_q = PaginationQuery {
            from: Some(message_id),
            to: query.to_end,
            dir: Some(PaginationDirection::F),
            limit: Some(limit),
        };

        // PERF: fetch in parallel
        // let before_res = data.message_list(channel_id, before_q);
        // let after_res = data.message_list(channel_id, after_q);
        // let message_res = data.message_get(channel_id, message_id);
        // let (before, after, message) = tokio::try_join!(before_res, after_res, message_res)?;

        let before = data.message_list(channel_id, before_q).await?;
        let after = data.message_list(channel_id, after_q).await?;
        let message = data.message_get(channel_id, message_id).await?;

        let mut items: Vec<Message> = before
            .items
            .into_iter()
            .chain(Some(message))
            .chain(after.items)
            .collect();

        self.populate_all(channel_id, user_id, &mut items).await?;

        Ok(ContextResponse {
            items,
            total: after.total,
            has_after: after.has_more,
            has_before: before.has_more,
        })
    }

    pub async fn list_versions(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        _user_id: Option<UserId>,
        pagination: PaginationQuery<MessageVerId>,
    ) -> Result<PaginationResponse<MessageVersion>> {
        let res = self
            .globals
            .begin_read()
            .await?
            .message_version_list(channel_id, message_id, pagination)
            .await?;

        Ok(res)
    }

    pub async fn get_version(
        &self,
        channel_id: ChannelId,
        version_id: MessageVerId,
        _user_id: Option<UserId>,
    ) -> Result<MessageVersion> {
        let message = self
            .globals
            .begin_read()
            .await?
            .message_version_get(channel_id, version_id)
            .await?;
        Ok(message)
    }

    pub async fn list_replies(
        &self,
        channel_id: ChannelId,
        root_message_id: Option<MessageId>,
        user_id: UserId,
        query: RepliesQuery,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<RepliesResponse> {
        let ancestors = match (query.context, root_message_id) {
            (Some(context), Some(start_id)) if context > 0 && pagination.from.is_none() => {
                self.message_reply_context(channel_id, Some(start_id), user_id, context)
                    .await?
            }
            _ => vec![],
        };

        let res = self
            .globals
            .begin_read()
            .await?
            .message_replies(
                channel_id,
                root_message_id,
                user_id,
                query.depth,
                query.breadth,
                pagination,
            )
            .await?;

        let mut messages = Vec::with_capacity(res.items.len());
        let mut counts = Vec::with_capacity(res.items.len());
        for mwc in res.items {
            messages.push(mwc.message);
            counts.push((mwc.count_direct, mwc.count_recursive));
        }

        self.populate_all(channel_id, Some(user_id), &mut messages)
            .await?;

        let mut items = Vec::with_capacity(messages.len());
        for (message, (count_direct, count_recursive)) in messages.into_iter().zip(counts) {
            items.push(MessageWithCounts {
                message,
                count_direct,
                count_recursive,
            });
        }

        let tree = TreeBuilder {
            messages: &items,
            max_depth: query.depth,
        }
        .build(root_message_id, 0);

        Ok(RepliesResponse { children: tree })
    }

    pub async fn list_pins(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>> {
        let res = self
            .globals
            .begin_read()
            .await?
            .message_pin_list(channel_id, user_id, pagination)
            .await?;
        self.process_message_list(channel_id, Some(user_id), res)
            .await
    }

    pub async fn list_activity(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>> {
        let res = self
            .globals
            .begin_read()
            .await?
            .message_list_activity(channel_id, user_id, pagination)
            .await?;
        self.process_message_list(channel_id, Some(user_id), res)
            .await
    }

    async fn fetch_media(
        &self,
        media_ref: Option<MediaReference>,
        user_id: UserId,
    ) -> Result<Option<Media>> {
        let Some(media_ref) = media_ref else {
            return Ok(None);
        };
        let Some(media_id) = media_ref.media_id() else {
            return Err(Error::Unimplemented);
        };
        let media = self
            .globals
            .begin_read()
            .await?
            .media_select(media_id)
            .await?;
        if media.user_id != Some(user_id) {
            return Err(Error::MissingPermissions);
        }
        Ok(Some(media))
    }

    async fn embed_from_create(&self, value: EmbedCreate, user_id: UserId) -> Result<Embed> {
        let media = self.fetch_media(value.media, user_id).await?;
        let thumbnail = self.fetch_media(value.thumbnail, user_id).await?;
        let author_avatar = self.fetch_media(value.author_avatar, user_id).await?;

        Ok(Embed {
            id: EmbedId::new(),
            ty: EmbedType::Custom,
            url: value.url,
            canonical_url: None,
            title: value.title,
            description: value.description,
            color: value
                .color
                .map(|s| csscolorparser::parse(&s)) // TODO: replace with `Color::from_str_strict` directly?
                .transpose()
                .map_err(|e| error!("Failed to parse color: {:?}", e))
                .ok()
                .flatten()
                .map(|c| Color::from_str_strict(&c.to_css_hex()))
                .transpose()?,
            media: media.map(|m| m.into()),
            thumbnail: thumbnail.map(|m| m.into()),
            author_name: value.author_name,
            author_url: value.author_url,
            author_avatar: author_avatar.map(|m| m.into()),
            site_name: None,
            site_avatar: None,
        })
    }
}

struct TreeBuilder<'a> {
    messages: &'a [MessageWithCounts],
    max_depth: u16,
}

impl<'a> TreeBuilder<'a> {
    fn new(messages: &'a [MessageWithCounts], max_depth: u16) -> Self {
        Self {
            messages,
            max_depth,
        }
    }

    fn build(&self, parent_id: Option<MessageId>, depth: u16) -> RepliesChildren {
        let children: Vec<_> = self
            .messages
            .iter()
            .filter(|msg| msg.message.reply_id() == parent_id)
            .map(|msg| {
                let (count_direct, count_recursive) = (msg.count_direct, msg.count_recursive);

                let subtree = if depth < self.max_depth {
                    self.build(Some(msg.message.id), depth + 1)
                } else {
                    RepliesChildren {
                        children: vec![],
                        count_direct,
                        count_recursive,
                        depth: (depth + 1) as u64,
                        cursor: None,
                        has_more: false,
                    }
                };

                RepliesMessage {
                    message: msg.message.clone(),
                    children: RepliesChildren {
                        count_direct,
                        count_recursive,
                        ..subtree
                    },
                }
            })
            .collect();

        RepliesChildren {
            count_direct: children.len() as u64,
            count_recursive: 0,
            children,
            depth: depth as u64,
            cursor: None,
            has_more: false,
        }
    }
}
