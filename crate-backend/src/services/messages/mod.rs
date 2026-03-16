use common::v1::types::emoji::EmojiOwner;
use common::v1::types::reaction::{ReactionCount, ReactionCounts, ReactionKey, ReactionKeyParam};
use common::v2::types::media::MediaReference;
use futures::{stream::FuturesUnordered, StreamExt};
use moka::future::Cache;
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tracing::error;

use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::misc::Color;
use common::v1::types::notifications::{Notification, NotificationType};
use common::v1::types::util::{Diff, Time};
use common::v1::types::{
    Channel, ChannelId, ChannelPatch, ContextQuery, ContextResponse, EmbedCreate, EmbedId,
    Mentions, MentionsChannel, MentionsEmoji, MentionsRole, MentionsUser, MessageCreate, MessageId,
    MessageSync, NotificationId, PaginationDirection, PaginationQuery, PaginationResponse,
    Permission, RepliesQuery, RoomId, User,
};
use common::v1::types::{MediaId, ThreadMemberPut, UserId};
use common::v2::types::embed::{Embed, EmbedType};
use common::v2::types::message::{
    Message, MessageAttachmentCreateType, MessageDefaultMarkdown, MessagePatch, MessageType,
    MessageVersion,
};
use http::StatusCode;
use linkify::LinkFinder;
use url::Url;
use validator::Validate;

use crate::routes::util::Auth;
use crate::services::notifications::preferences::NotificationAction;
use crate::types::{DbMessageCreate, DbMessageUpdate, MediaLinkType, MentionsIds, MessageVerId};
use crate::{Error, Result, ServerStateInner};

pub mod mentions;

pub struct ServiceMessages {
    state: Arc<ServerStateInner>,
    pub idempotency_keys: Cache<String, Message>,
}

impl ServiceMessages {
    pub async fn get(
        &self,
        thread_id: ChannelId,
        message_id: MessageId,
        user_id: UserId,
    ) -> Result<Message> {
        let mut message = self
            .state
            .data()
            .message_get(thread_id, message_id, user_id)
            .await?;
        self.state.presign_message(&mut message).await?;

        self.populate_all(thread_id, user_id, std::slice::from_mut(&mut message))
            .await?;

        Ok(message)
    }

    pub async fn get_many(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        message_ids: &[MessageId],
    ) -> Result<Vec<Message>> {
        let mut messages = self
            .state
            .data()
            .message_get_many(channel_id, message_ids, user_id)
            .await?;

        for message in &mut messages {
            self.state.presign_message(message).await?;
        }

        self.populate_all(channel_id, user_id, &mut messages)
            .await?;

        Ok(messages)
    }

    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            idempotency_keys: Cache::builder()
                .time_to_live(Duration::from_secs(300))
                .build(),
        }
    }

    fn handle_url_embed(
        &self,
        message: Message,
        user_id: UserId,
        content: String,
    ) -> impl Future<Output = ()> + Send + 'static {
        let s = self.state.clone();
        let srv = s.services();
        async move {
            let links: Vec<_> = LinkFinder::new().links(&content).collect();
            for link in links {
                if let Some(url) = link.as_str().parse::<Url>().ok() {
                    if let Err(e) = srv
                        .embed
                        .queue(
                            Some(crate::types::MessageRef {
                                thread_id: message.channel_id,
                                message_id: message.id,
                                version_id: message.latest_version.version_id,
                            }),
                            user_id,
                            url,
                        )
                        .await
                    {
                        error!("Failed to queue embed generation: {:?}", e);
                    }
                }
            }
        }
    }

    pub async fn create(
        &self,
        thread_id: ChannelId,
        auth: &Auth,
        nonce: Option<String>,
        json: MessageCreate,
        header_timestamp: Option<Time>,
    ) -> Result<Message> {
        if let Some(n) = &nonce {
            self.idempotency_keys
                .try_get_with(
                    n.clone(),
                    self.create_inner2(
                        thread_id,
                        auth.user.id,
                        Some(auth),
                        nonce,
                        json,
                        header_timestamp,
                    ),
                )
                .await
                .map_err(|err| err.fake_clone())
        } else {
            self.create_inner2(
                thread_id,
                auth.user.id,
                Some(auth),
                nonce,
                json,
                header_timestamp,
            )
            .await
        }
    }

    pub async fn create_system(
        &self,
        thread_id: ChannelId,
        user_id: UserId,
        nonce: Option<String>,
        json: MessageCreate,
    ) -> Result<Message> {
        if let Some(n) = &nonce {
            self.idempotency_keys
                .try_get_with(
                    n.clone(),
                    self.create_inner2(thread_id, user_id, None, nonce, json, None),
                )
                .await
                .map_err(|err| err.fake_clone())
        } else {
            self.create_inner2(thread_id, user_id, None, nonce, json, None)
                .await
        }
    }

    async fn check_timestamp_override(
        &self,
        user_id: UserId,
        thread_id: ChannelId,
        timestamp: Time,
    ) -> Result<Time> {
        let user = self.state.data().user_get(user_id).await?;
        let srv = self.state.services();

        let owner_id = if let Some(puppet) = user.puppet {
            puppet.owner_id.into_inner().into()
        } else if user.bot {
            let app = self
                .state
                .data()
                .application_get(user_id.into_inner().into())
                .await
                .map_err(|_| {
                    Error::BadStatic("MemberBridge permission required to override timestamp")
                })?;
            app.owner_id.into_inner().into()
        } else {
            return Err(Error::BadStatic(
                "MemberBridge permission required to override timestamp",
            ));
        };

        let owner_perms = srv.perms.for_channel(owner_id, thread_id).await?;
        owner_perms.ensure_all(&[Permission::ViewChannel, Permission::MemberBridge])?;
        Ok(timestamp)
    }

    fn extract_and_validate_media(
        json: &MessageCreate,
    ) -> Result<(Vec<MediaId>, HashSet<MediaId>)> {
        let mut all_media_ids = HashSet::new();

        let attachment_ids: Vec<_> = json
            .attachments
            .iter()
            .filter_map(|r| match &r.ty {
                MessageAttachmentCreateType::Media { media, .. } => match media {
                    MediaReference::Media { media_id } => Some(*media_id),
                    MediaReference::Url { .. } => None,
                    MediaReference::Attachment { .. } => None,
                },
            })
            .collect();

        for id in &attachment_ids {
            if !all_media_ids.insert(*id) {
                return Err(Error::BadStatic("duplicate media id in request"));
            }
        }

        for embed in &json.embeds {
            if let Some(m) = &embed.media {
                let Some(media_id) = m.media_id() else {
                    return Err(Error::Unimplemented);
                };
                if !all_media_ids.insert(media_id) {
                    return Err(Error::BadStatic("duplicate media id in request"));
                }
            }
            if let Some(m) = &embed.thumbnail {
                let Some(media_id) = m.media_id() else {
                    return Err(Error::Unimplemented);
                };
                if !all_media_ids.insert(media_id) {
                    return Err(Error::BadStatic("duplicate media id in request"));
                }
            }
            if let Some(m) = &embed.author_avatar {
                let Some(media_id) = m.media_id() else {
                    return Err(Error::Unimplemented);
                };
                if !all_media_ids.insert(media_id) {
                    return Err(Error::BadStatic("duplicate media id in request"));
                }
            }
        }

        Ok((attachment_ids, all_media_ids))
    }

    // TODO: refactor create and edit together
    // FIXME: webhook permisison checks
    pub async fn edit(
        &self,
        thread_id: ChannelId,
        message_id: MessageId,
        user_id: UserId,
        json: MessagePatch,
        header_timestamp: Option<Time>,
    ) -> Result<(StatusCode, Message)> {
        let s = &self.state;
        json.validate()?;
        let data = s.data();
        let srv = s.services();
        let user = srv.users.get(user_id, None).await?;
        let thread = srv.channels.get(thread_id, Some(user_id)).await?;
        let is_webhook = user.webhook.is_some();

        let created_at = if let Some(ts) = header_timestamp {
            Some(
                self.check_timestamp_override(user_id, thread_id, ts)
                    .await?,
            )
        } else {
            None
        };

        let perms = if is_webhook {
            None
        } else {
            Some(srv.perms.for_channel(user_id, thread_id).await?)
        };

        if let Some(perms) = &perms {
            perms.ensure(Permission::ViewChannel)?;
        }

        let can_use_external_emoji = if !is_webhook {
            if let Some(perms) = &perms {
                perms.has(Permission::EmojiUseExternal)
            } else {
                true // system
            }
        } else {
            true
        };

        let mut message = match self.get(thread_id, message_id, user_id).await {
            Ok(m) => m,
            Err(e) => {
                if is_webhook {
                    return Err(Error::ApiError(ApiError::from_code(
                        ErrorCode::UnknownMessage,
                    )));
                }
                return Err(e);
            }
        };

        if !message.latest_version.message_type.is_editable() {
            return Err(Error::BadStatic("cant edit that message"));
        }
        if message.author_id != user_id {
            if is_webhook {
                return Err(Error::ApiError(ApiError::from_code(
                    ErrorCode::UnknownMessage,
                )));
            }
            return Err(ApiError::from_code(ErrorCode::MissingPermissions).into());
        }
        if json.content.is_none()
            && json.attachments.as_ref().is_some_and(|a| a.is_empty())
            && json.embeds.as_ref().is_some_and(|a| a.is_empty())
        {
            return Err(Error::BadStatic(
                "at least one of content, attachments, or embeds must be defined",
            ));
        }

        if let Some(perms) = &perms {
            let mut required_perms = vec![];
            if json.attachments.as_ref().is_some_and(|a| !a.is_empty()) {
                required_perms.push(Permission::MessageAttachments);
            }
            if json.embeds.as_ref().is_some_and(|a| !a.is_empty()) {
                required_perms.push(Permission::MessageEmbeds);
            }
            perms.ensure_all(&required_perms)?;
        }

        if !json.changes(&message) {
            return Ok((StatusCode::NOT_MODIFIED, message));
        }
        let attachment_ids: Vec<_> = json
            .attachments
            .clone()
            .map(|ats| {
                ats.into_iter()
                    .filter_map(|r| match r.ty {
                        MessageAttachmentCreateType::Media { media, .. } => match media {
                            MediaReference::Media { media_id } => Some(media_id),
                            _ => None,
                        },
                    })
                    .collect()
            })
            .unwrap_or_else(|| match &message.latest_version.message_type {
                MessageType::DefaultMarkdown(msg) => msg
                    .attachments
                    .iter()
                    .filter_map(|a| match &a.ty {
                        common::v2::types::message::MessageAttachmentType::Media { media } => {
                            Some(media.id)
                        }
                    })
                    .collect(),
                _ => vec![],
            });
        for id in &attachment_ids {
            data.media_select(*id).await?;
            let existing = data.media_link_select(*id).await?;
            let has_link = existing.iter().any(|i| {
                i.link_type == MediaLinkType::Message && i.target_id == message_id.into_inner()
            });
            if !has_link {
                return Err(Error::BadStatic("cant reuse media"));
            }
        }
        let mut embeds = vec![];
        if let Some(embed_creates) = json.embeds.clone() {
            let mut embed_futs = Vec::new();
            for embed_create in embed_creates {
                let v1_embed_create: common::v1::types::EmbedCreate = embed_create.into();
                embed_futs.push(self.embed_from_create(v1_embed_create, user_id));
            }
            embeds = futures_util::future::try_join_all(embed_futs).await?;
        }

        if let Some(room_id) = thread.room_id {
            let automod = srv.automod.load(room_id).await?;
            let scan = automod.scan_message_update(&message, &json);
            if scan.is_triggered() {
                let removed = srv
                    .automod
                    .enforce_message_create(room_id, thread_id, message_id, user_id, &scan)
                    .await?;
                if removed {
                    data.message_remove_bulk(thread_id, &[message_id]).await?;
                }
            }
        }

        let (content, payload, mentions) = match message.latest_version.message_type.clone() {
            MessageType::DefaultMarkdown(msg) => {
                let mut content = json.content.as_ref().cloned().unwrap_or(msg.content);
                let mut mentions = message.latest_version.mentions.clone();

                if json.content.is_some() {
                    let parsed_mentions = mentions::parse(
                        content.as_deref().unwrap_or_default(),
                        &Default::default(),
                    );
                    mentions = self
                        .fetch_full_mentions_from_ids(parsed_mentions, thread.room_id)
                        .await?;
                }

                if let Some(room_id) = thread.room_id {
                    if let Some(c) = &mut content {
                        *c = self
                            .enforce_emoji_use_external(
                                &mentions,
                                room_id,
                                can_use_external_emoji,
                                c,
                            )
                            .await?;
                    }
                }

                Result::Ok((
                    content.clone(),
                    MessageType::DefaultMarkdown(MessageDefaultMarkdown {
                        content,
                        attachments: vec![],
                        embeds: embeds.clone(),
                        metadata: None,
                        reply_id: json.reply_id.unwrap_or(msg.reply_id),
                    }),
                    mentions,
                ))
            }
            _ => return Err(Error::Unimplemented),
        }?;

        let version_id = data
            .message_update(
                thread_id,
                message_id,
                DbMessageUpdate {
                    attachment_ids: attachment_ids.clone(),
                    author_id: user_id,
                    embeds: embeds.into_iter().map(|e| e.into()).collect(),
                    message_type: payload,
                    created_at: created_at.map(|t| t.into()),
                    mentions,
                },
            )
            .await?;

        for id in &attachment_ids {
            data.media_link_insert(*id, *version_id, MediaLinkType::MessageVersion)
                .await?;
            data.media_link_insert(*id, *message_id, MediaLinkType::Message)
                .await?;
        }

        let ver = data
            .message_version_get(thread_id, version_id, user_id)
            .await?;

        message.latest_version = ver;

        if let Some(content) = &content {
            let can_embed = if let Some(perms) = &perms {
                perms.has(Permission::MessageEmbeds)
            } else {
                is_webhook
            };
            if can_embed {
                tokio::spawn(self.handle_url_embed(message.clone(), user_id, content.clone()));
            }
        }

        self.populate_all(thread_id, user_id, std::slice::from_mut(&mut message))
            .await?;

        s.presign_message(&mut message).await?;
        s.broadcast_channel(
            thread_id,
            user_id,
            MessageSync::MessageUpdate {
                message: message.clone(),
            },
        )
        .await?;
        s.services().channels.invalidate(thread_id).await; // last version id
        Ok((StatusCode::OK, message))
    }

    pub async fn fetch_full_mentions_from_ids(
        &self,
        mentions_ids: MentionsIds,
        room_id: Option<RoomId>,
    ) -> Result<Mentions> {
        let srv = self.state.services();

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
        let srv = self.state.services();
        let mut allowed_emoji = vec![];

        let emoji_ids: Vec<_> = m.emojis.iter().map(|e| e.id).collect();
        let emojis = srv.cache.emoji_get_many(&emoji_ids).await?;

        for emoji in emojis {
            let is_room_emoji = emoji.owner == Some(EmojiOwner::Room { room_id });
            if is_room_emoji || allow {
                allowed_emoji.push(emoji.id);
            }
        }

        Ok(mentions::strip_emoji(content, &allowed_emoji))
    }

    async fn fetch_mentions_data(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        messages: &[Message],
    ) -> Result<Vec<Mentions>> {
        if messages.is_empty() {
            return Ok(vec![]);
        }

        let data = self.state.data();
        let channel = self
            .state
            .services()
            .channels
            .get(channel_id, Some(user_id))
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
        user_id: UserId,
        messages: &[Message],
    ) -> Result<HashMap<ChannelId, Channel>> {
        let mut threads_map = HashMap::new();

        let srv = self.state.services();
        let mut thread_futs: FuturesUnordered<_> = messages
            .iter()
            .map(|m| {
                let srv2 = Arc::clone(&srv);
                let cid: ChannelId = (*m.id).into();
                async move {
                    let thread = srv2.channels.get(cid, Some(user_id)).await;
                    (cid, thread)
                }
            })
            .collect();

        while let Some((id, thread_result)) = thread_futs.next().await {
            if let Ok(thread) = thread_result {
                threads_map.insert(id, thread);
            }
        }

        Ok(threads_map)
    }

    async fn fetch_reactions_data(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        messages: &[Message],
    ) -> Result<HashMap<MessageId, ReactionCounts>> {
        let data = self.state.data();
        let message_ids: Vec<MessageId> = messages.iter().map(|m| m.id).collect();
        let reactions = data
            .reaction_fetch_all(channel_id, user_id, &message_ids)
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

    pub async fn populate_all(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        messages: &mut [Message],
    ) -> Result<()> {
        if messages.is_empty() {
            return Ok(());
        }

        let mentions_fut = self.fetch_mentions_data(channel_id, user_id, messages);
        let threads_fut = self.fetch_threads_data(user_id, messages);
        let reactions_fut = self.fetch_reactions_data(channel_id, user_id, messages);

        let (mentions_data, threads_data, reactions_data) =
            tokio::try_join!(mentions_fut, threads_fut, reactions_fut)?;

        for (i, message) in messages.iter_mut().enumerate() {
            if let Some(m) = mentions_data.get(i) {
                message.latest_version.mentions = m.clone();
            }
            let thread_channel_id: ChannelId = (*message.id).into();
            if let Some(t) = threads_data.get(&thread_channel_id) {
                message.thread = Some(Box::new(t.clone()));
            }
            if let Some(r) = reactions_data.get(&message.id) {
                message.reactions = r.clone();
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
            .state
            .services()
            .perms
            .for_channel(user_id, channel_id)
            .await?;
        perms.ensure(Permission::ViewChannel)?;

        let data = self.state.data();
        let mut ancestors = data
            .message_get_ancestors(start_message_id, context)
            .await?;

        for message in &mut ancestors {
            self.state.presign_message(message).await?;
        }

        self.populate_all(channel_id, user_id, &mut ancestors)
            .await?;

        Ok(ancestors)
    }

    async fn process_message_list(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        mut res: PaginationResponse<Message>,
    ) -> Result<PaginationResponse<Message>> {
        self.populate_all(channel_id, user_id, &mut res.items)
            .await?;

        for message in &mut res.items {
            self.state.presign_message(message).await?;
        }

        Ok(res)
    }

    pub async fn list(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>> {
        let res = self
            .state
            .data()
            .message_list(channel_id, user_id, pagination)
            .await?;
        self.process_message_list(channel_id, user_id, res).await
    }

    pub async fn list_deleted(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>> {
        let res = self
            .state
            .data()
            .message_list_deleted(channel_id, user_id, pagination)
            .await?;
        self.process_message_list(channel_id, user_id, res).await
    }

    pub async fn list_removed(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>> {
        let res = self
            .state
            .data()
            .message_list_removed(channel_id, user_id, pagination)
            .await?;
        self.process_message_list(channel_id, user_id, res).await
    }

    pub async fn list_all(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>> {
        let res = self
            .state
            .data()
            .message_list_all(channel_id, user_id, pagination)
            .await?;
        self.process_message_list(channel_id, user_id, res).await
    }

    pub async fn list_context(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        user_id: UserId,
        query: ContextQuery,
    ) -> Result<ContextResponse> {
        let s = &self.state;
        let data = s.data();

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

        let (before_res, after_res, message_res) = tokio::join!(
            data.message_list(channel_id, user_id, before_q),
            data.message_list(channel_id, user_id, after_q),
            data.message_get(channel_id, message_id, user_id)
        );

        let before = before_res?;
        let after = after_res?;
        let message = message_res.ok();

        let mut items: Vec<Message> = before
            .items
            .into_iter()
            .chain(message)
            .chain(after.items)
            .collect();

        self.populate_all(channel_id, user_id, &mut items).await?;

        for item in &mut items {
            s.presign_message(item).await?;
        }

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
        user_id: UserId,
        pagination: PaginationQuery<MessageVerId>,
    ) -> Result<PaginationResponse<MessageVersion>> {
        let s = &self.state;
        let data = s.data();
        let mut res = data
            .message_version_list(channel_id, message_id, user_id, pagination)
            .await?;

        for message in &mut res.items {
            s.presign_message_version(message).await?;
        }

        Ok(res)
    }

    pub async fn get_version(
        &self,
        channel_id: ChannelId,
        version_id: MessageVerId,
        user_id: UserId,
    ) -> Result<MessageVersion> {
        let s = &self.state;
        let data = s.data();
        let mut message = data
            .message_version_get(channel_id, version_id, user_id)
            .await?;
        s.presign_message_version(&mut message).await?;
        Ok(message)
    }

    pub async fn list_replies(
        &self,
        channel_id: ChannelId,
        root_message_id: Option<MessageId>,
        user_id: UserId,
        query: RepliesQuery,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>> {
        let mut ancestors = match (query.context, root_message_id) {
            (Some(context), Some(start_id)) if context > 0 && pagination.from.is_none() => {
                self.message_reply_context(channel_id, Some(start_id), user_id, context)
                    .await?
            }
            _ => vec![],
        };

        let s = &self.state;
        let data = s.data();
        let mut res = data
            .message_replies(
                channel_id,
                root_message_id,
                user_id,
                query.depth,
                query.breadth,
                pagination,
            )
            .await?;

        self.populate_all(channel_id, user_id, &mut res.items)
            .await?;

        for message in &mut res.items {
            s.presign_message(message).await?;
        }

        if !ancestors.is_empty() {
            // NOTE: maybe i don't want to include this, since the ancestors/context aren't part of replies?
            // res.total += ancestors.len() as u64;
            res.total += ancestors.len() as u64;
            // make sure ancestors come first
            ancestors.append(&mut res.items);
            res.items = ancestors;
        }

        Ok(res)
    }

    pub async fn list_pins(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>> {
        let res = self
            .state
            .data()
            .message_pin_list(channel_id, user_id, pagination)
            .await?;
        self.process_message_list(channel_id, user_id, res).await
    }

    pub async fn list_activity(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>> {
        let res = self
            .state
            .data()
            .message_list_activity(channel_id, user_id, pagination)
            .await?;
        self.process_message_list(channel_id, user_id, res).await
    }

    async fn fetch_media(
        &self,
        media_ref: Option<MediaReference>,
        user_id: UserId,
    ) -> Result<Option<common::v2::types::media::Media>> {
        let Some(media_ref) = media_ref else {
            return Ok(None);
        };
        let Some(media_id) = media_ref.media_id() else {
            return Err(Error::Unimplemented);
        };
        let media = self.state.data().media_select(media_id).await?;
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
                .map(|s| csscolorparser::parse(&s))
                .transpose()
                .map_err(|e| error!("Failed to parse color: {:?}", e))
                .ok()
                .flatten()
                .map(|c| Color::from_hex_string(c.to_css_hex())),
            media: media.map(|m| m.into()),
            thumbnail: thumbnail.map(|m| m.into()),
            author_name: value.author_name,
            author_url: value.author_url,
            author_avatar: author_avatar.map(|m| m.into()),
            site_name: None,
            site_avatar: None,
        })
    }

    async fn create_inner2(
        &self,
        thread_id: ChannelId,
        user_id: UserId,
        auth: Option<&Auth>,
        nonce: Option<String>,
        json: MessageCreate,
        header_timestamp: Option<Time>,
    ) -> Result<Message> {
        json.validate()?;
        let data = self.state.data();
        let user = data.user_get(user_id).await?;
        let chan = self
            .state
            .services()
            .channels
            .get(thread_id, Some(user_id))
            .await?;
        let is_webhook = user.webhook.is_some();

        // 1. Pre-flight checks
        let created_at = if let Some(ts) = header_timestamp {
            Some(
                self.check_timestamp_override(user_id, thread_id, ts)
                    .await?,
            )
        } else {
            None
        };

        let can_use_external_emoji = self
            .enforce_send_permissions(auth, &user, &chan, &json)
            .await?;
        let (attachment_ids, all_media_ids) = self.validate_and_claim_media(&json).await?;

        // 2. Prepare payload (Embeds, Automod, Mentions)
        let embeds = self.build_embeds(json.embeds.clone(), user_id).await?;
        let removed_at = self.enforce_automod(&chan, &json, user_id).await?;
        let (content, mentions) = self
            .process_mentions_and_emojis(&json, &chan, can_use_external_emoji)
            .await?;

        // 3. Database Insertion
        let payload = MessageType::DefaultMarkdown(MessageDefaultMarkdown {
            content: content.clone(),
            attachments: vec![],
            embeds: embeds.into_iter().map(|e| e.into()).collect(),
            metadata: None,
            reply_id: json.reply_id,
        });

        let message_id = MessageId::new();
        let _message_uuid = self
            .insert_message_to_db(
                message_id,
                thread_id,
                user_id,
                payload,
                created_at,
                removed_at,
                &mentions,
                &all_media_ids,
                &attachment_ids,
            )
            .await?;

        // 4. Post-processing & Cache updates
        let mut message = self.get(thread_id, message_id, user_id).await?;
        message.latest_version.mentions = mentions.clone();
        self.ensure_thread_membership(thread_id, user_id, chan.room_id)
            .await?;

        if let Some(c) = content {
            self.spawn_url_unfurling(message.clone(), user_id, c, is_webhook)
                .await;
        }

        // 5. Broadcast & Notify
        self.state
            .broadcast_channel_with_nonce(
                thread_id,
                user_id,
                nonce.as_deref(),
                MessageSync::MessageCreate {
                    message: message.clone(),
                },
            )
            .await?;
        self.state.services().channels.invalidate(thread_id).await;

        // THIS is the biggest win: moving the 150-line tokio::spawn block out!
        self.dispatch_notifications(message.clone(), chan.clone(), mentions, user_id)
            .await;

        Ok(message)
    }

    async fn enforce_send_permissions(
        &self,
        auth: Option<&Auth>,
        user: &User,
        thread: &Channel,
        json: &MessageCreate,
    ) -> Result<bool> {
        // Webhooks bypass
        if user.webhook.is_some() {
            return Ok(true);
        }

        let srv = self.state.services();
        let data = self.state.data();

        // System messages bypass permissions but still handle archived channels
        let Some(auth) = auth else {
            if thread.is_archived() {
                data.channel_update(
                    thread.id,
                    ChannelPatch {
                        archived: Some(false),
                        ..Default::default()
                    },
                )
                .await?;
                srv.channels.invalidate(thread.id).await;
                let channel = srv.channels.get(thread.id, None).await?;
                self.state
                    .broadcast_channel(
                        thread.id,
                        user.id,
                        MessageSync::ChannelUpdate {
                            channel: Box::new(channel),
                        },
                    )
                    .await?;
            }
            return Ok(true);
        };

        let perms = srv.perms.for_channel(user.id, thread.id).await?;
        perms.ensure_unlocked()?;

        // Build required perms array dynamically
        let mut required = vec![Permission::ViewChannel];
        required.push(if thread.is_thread() {
            Permission::MessageCreateThread
        } else {
            Permission::MessageCreate
        });
        if !json.attachments.is_empty() {
            required.push(Permission::MessageAttachments);
        }
        if !json.embeds.is_empty() {
            required.push(Permission::MessageEmbeds);
        }
        perms.ensure_all(&required)?;

        // Handle Slowmode logic
        if !perms.can_bypass_slowmode() {
            if let Some(message_slowmode_expire_at) = data
                .channel_get_message_slowmode_expire_at(thread.id, user.id)
                .await?
            {
                if message_slowmode_expire_at > Time::now_utc() {
                    return Err(Error::BadStatic("slowmode in effect"));
                }
            }

            if let Some(slowmode_delay) = thread.slowmode_message {
                let next_message_time =
                    Time::now_utc() + std::time::Duration::from_secs(slowmode_delay);
                data.channel_set_message_slowmode_expire_at(thread.id, user.id, next_message_time)
                    .await?;
            }
        }

        // Handle Unarchiving
        if thread.is_archived() {
            srv.channels
                .update(
                    auth,
                    thread.id,
                    ChannelPatch {
                        archived: Some(false),
                        ..Default::default()
                    },
                )
                .await?;
        }

        Ok(perms.has(Permission::EmojiUseExternal))
    }

    async fn validate_and_claim_media(
        &self,
        json: &MessageCreate,
    ) -> Result<(Vec<MediaId>, HashSet<MediaId>)> {
        let (attachment_ids, all_media_ids) = Self::extract_and_validate_media(json)?;

        let data = self.state.data();
        for id in &all_media_ids {
            data.media_select(*id).await?;
            let existing = data.media_link_select(*id).await?;
            if existing
                .iter()
                .any(|l| l.link_type == MediaLinkType::Message)
            {
                return Err(Error::BadStatic("cant reuse media"));
            }
        }
        Ok((attachment_ids, all_media_ids))
    }

    async fn build_embeds(
        &self,
        embeds_create: Vec<common::v1::types::EmbedCreate>,
        user_id: UserId,
    ) -> Result<Vec<Embed>> {
        if embeds_create.is_empty() {
            return Ok(vec![]);
        }

        let mut embed_futs = Vec::new();
        for embed_create in embeds_create {
            embed_futs.push(self.embed_from_create(embed_create, user_id));
        }
        futures_util::future::try_join_all(embed_futs).await
    }

    async fn enforce_automod(
        &self,
        chan: &Channel,
        json: &MessageCreate,
        user_id: UserId,
    ) -> Result<Option<Time>> {
        let Some(room_id) = chan.room_id else {
            return Ok(None);
        };

        let srv = self.state.services();
        let automod = srv.automod.load(room_id).await?;
        let scan = automod.scan_message_create(json);

        if scan.is_triggered() {
            let message_id = MessageId::new();
            let removed = srv
                .automod
                .enforce_message_create(room_id, chan.id, message_id, user_id, &scan)
                .await?;
            if removed {
                return Ok(Some(Time::now_utc()));
            }
        }
        Ok(None)
    }

    async fn process_mentions_and_emojis(
        &self,
        json: &MessageCreate,
        chan: &Channel,
        can_use_external_emoji: bool,
    ) -> Result<(Option<String>, Mentions)> {
        let content = json.content.clone();
        let parsed_mentions =
            mentions::parse(content.as_deref().unwrap_or_default(), &json.mentions);
        let mentions = self
            .fetch_full_mentions_from_ids(parsed_mentions, chan.room_id)
            .await?;

        let mut final_content = content;
        if let Some(room_id) = chan.room_id {
            if let Some(c) = &mut final_content {
                *c = self
                    .enforce_emoji_use_external(&mentions, room_id, can_use_external_emoji, c)
                    .await?;
            }
        }

        Ok((final_content, mentions))
    }

    async fn insert_message_to_db(
        &self,
        message_id: MessageId,
        channel_id: ChannelId,
        author_id: UserId,
        payload: MessageType,
        created_at: Option<Time>,
        removed_at: Option<Time>,
        mentions: &Mentions,
        all_media_ids: &HashSet<MediaId>,
        attachment_ids: &[MediaId],
    ) -> Result<uuid::Uuid> {
        let data = self.state.data();

        let message_id_db = data
            .message_create(DbMessageCreate {
                id: Some(message_id),
                channel_id,
                attachment_ids: attachment_ids.to_vec(),
                author_id,
                embeds: match payload {
                    MessageType::DefaultMarkdown(ref md) => {
                        md.embeds.iter().cloned().map(|e| e.into()).collect()
                    }
                    _ => vec![],
                },
                message_type: payload,
                created_at: created_at.map(|t| t.into()),
                removed_at: removed_at.map(|t| t.into()),
                mentions: mentions.clone(),
            })
            .await?;
        let message_uuid = message_id_db.into_inner();

        if message_id != message_id_db {
            error!("Message id mismatch: {} != {}", message_id, message_id_db);
        }

        for id in all_media_ids {
            data.media_link_insert(*id, message_uuid, MediaLinkType::Message)
                .await?;
            data.media_link_insert(*id, message_uuid, MediaLinkType::MessageVersion)
                .await?;
        }

        Ok(message_uuid)
    }

    async fn ensure_thread_membership(
        &self,
        thread_id: ChannelId,
        user_id: UserId,
        room_id: Option<common::v1::types::RoomId>,
    ) -> Result<()> {
        let data = self.state.data();
        let tm = data.thread_member_get(thread_id, user_id).await;
        if tm.is_err() {
            data.thread_member_put(thread_id, user_id, ThreadMemberPut::default())
                .await?;
            let thread_member = data.thread_member_get(thread_id, user_id).await?;
            let msg = MessageSync::ThreadMemberUpsert {
                room_id,
                thread_id,
                added: vec![thread_member],
                removed: vec![],
            };
            self.state
                .broadcast_channel(thread_id, user_id, msg)
                .await?;
        }
        Ok(())
    }

    async fn spawn_url_unfurling(
        &self,
        message: Message,
        user_id: UserId,
        content: String,
        is_webhook: bool,
    ) {
        let mut should_embed = is_webhook;
        if !should_embed {
            if let Ok(perms) = self
                .state
                .services()
                .perms
                .for_channel(user_id, message.channel_id)
                .await
            {
                should_embed = perms.has(Permission::MessageEmbeds);
            }
        }

        if should_embed {
            tokio::spawn(self.handle_url_embed(message, user_id, content));
        }
    }

    async fn dispatch_notifications(
        &self,
        message: Message,
        chan: Channel,
        mentions: Mentions,
        author_id: UserId,
    ) {
        let s_clone = self.state.clone();
        let channel_id = message.channel_id;
        let message_id = message.id;
        let version_id = message.latest_version.version_id;
        let room_id = chan.room_id;
        let channel_is_thread = chan.is_thread();

        tokio::spawn(async move {
            let mut notified_users = HashSet::new();

            // Direct user mentions
            for u in mentions.users {
                if u.id == author_id {
                    continue;
                }

                if channel_is_thread {
                    let member = s_clone.data().thread_member_get(channel_id, u.id).await;
                    if member.is_err() {
                        if s_clone
                            .data()
                            .thread_member_put(channel_id, u.id, Default::default())
                            .await
                            .is_ok()
                        {
                            if let Ok(thread_member) =
                                s_clone.data().thread_member_get(channel_id, u.id).await
                            {
                                let msg = MessageSync::ThreadMemberUpsert {
                                    room_id,
                                    thread_id: channel_id,
                                    added: vec![thread_member],
                                    removed: vec![],
                                };
                                if let Err(e) =
                                    s_clone.broadcast_channel(channel_id, author_id, msg).await
                                {
                                    error!("Failed to broadcast thread member upsert: {}", e);
                                }
                            }
                        }
                    }
                }

                if notified_users.insert(u.id) {
                    if let Err(e) = s_clone
                        .data()
                        .unread_increment_mentions(u.id, channel_id, message_id, version_id, 1)
                        .await
                    {
                        error!("Failed to increment mention count for user {}: {}", u.id, e);
                    }

                    let room_id = s_clone
                        .services()
                        .channels
                        .get(channel_id, Some(u.id))
                        .await
                        .ok()
                        .and_then(|ch| ch.room_id);

                    let notification = Notification {
                        id: NotificationId::new(),
                        ty: NotificationType::Message {
                            room_id: room_id,
                            channel_id,
                            message_id,
                        },
                        added_at: Time::now_utc(),
                        read_at: None,
                        note: None,
                    };
                    let action = s_clone
                        .services()
                        .notifications
                        .calculator(u.id, &notification)
                        .action()
                        .await
                        .unwrap_or(NotificationAction::Skip);

                    if action.should_add_to_inbox() {
                        if let Err(e) = s_clone.data().notification_add(u.id, notification).await {
                            error!(
                                "Failed to add mention notification for user {}: {}",
                                u.id, e
                            );
                        }
                    }
                }
            }

            // Role mentions
            if let Some(_room_id) = room_id {
                for r in mentions.roles {
                    let role_id = r.id;
                    if let Ok(members) = s_clone
                        .data()
                        .role_member_list(role_id, Default::default())
                        .await
                    {
                        if channel_is_thread
                            && members.items.len()
                                < crate::consts::MAX_ROLE_MENTION_MEMBERS_ADD as usize
                        {
                            for member in &members.items {
                                if let Err(e) = s_clone
                                    .data()
                                    .thread_member_put(
                                        channel_id,
                                        member.user_id,
                                        Default::default(),
                                    )
                                    .await
                                {
                                    error!(
                                        "Failed to add mentioned role member {} to thread {}: {}",
                                        member.user_id, channel_id, e
                                    );
                                }
                            }
                        }

                        for member in members.items {
                            if member.user_id == author_id {
                                continue;
                            }
                            if notified_users.insert(member.user_id) {
                                if let Err(e) = s_clone
                                    .data()
                                    .unread_increment_mentions(
                                        member.user_id,
                                        channel_id,
                                        message_id,
                                        version_id,
                                        1,
                                    )
                                    .await
                                {
                                    error!(
                                        "Failed to increment mention count for user {}: {}",
                                        member.user_id, e
                                    );
                                }

                                let room_id = s_clone
                                    .services()
                                    .channels
                                    .get(channel_id, Some(member.user_id))
                                    .await
                                    .ok()
                                    .and_then(|ch| ch.room_id);

                                let notification = Notification {
                                    id: NotificationId::new(),
                                    ty: NotificationType::Message {
                                        room_id,
                                        channel_id,
                                        message_id,
                                    },
                                    added_at: Time::now_utc(),
                                    read_at: None,
                                    note: None,
                                };
                                let action = s_clone
                                    .services()
                                    .notifications
                                    .calculator(member.user_id, &notification)
                                    .action()
                                    .await
                                    .unwrap_or(NotificationAction::Push);

                                if action.should_add_to_inbox() {
                                    if let Err(e) = s_clone
                                        .data()
                                        .notification_add(member.user_id, notification)
                                        .await
                                    {
                                        error!(
                                            "Failed to add role mention notification for user {}: {}",
                                            member.user_id, e
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // @everyone mentions
            if mentions.everyone {
                let mut users_to_notify = Vec::new();
                if channel_is_thread {
                    if let Ok(members) = s_clone.data().thread_member_list_all(channel_id).await {
                        users_to_notify.extend(members.into_iter().map(|m| m.user_id));
                    }
                } else if let Some(room_id) = room_id {
                    if let Ok(members) = s_clone.data().room_member_list_all(room_id).await {
                        users_to_notify.extend(members.into_iter().map(|m| m.user_id));
                    }
                }

                for user_id in users_to_notify {
                    if user_id == author_id {
                        continue;
                    }
                    if notified_users.insert(user_id) {
                        if let Err(e) = s_clone
                            .data()
                            .unread_increment_mentions(
                                user_id, channel_id, message_id, version_id, 1,
                            )
                            .await
                        {
                            error!(
                                "Failed to increment mention count for user {}: {}",
                                user_id, e
                            );
                        }
                        let room_id = s_clone
                            .services()
                            .channels
                            .get(channel_id, Some(user_id))
                            .await
                            .ok()
                            .and_then(|ch| ch.room_id);

                        let notification = Notification {
                            id: NotificationId::new(),
                            ty: NotificationType::Message {
                                room_id,
                                channel_id,
                                message_id,
                            },
                            added_at: Time::now_utc(),
                            read_at: None,
                            note: None,
                        };
                        if let Err(e) = s_clone.data().notification_add(user_id, notification).await
                        {
                            error!(
                                "Failed to add everyone mention notification for user {}: {}",
                                user_id, e
                            );
                        }
                    }
                }
            }
        });
    }
}
