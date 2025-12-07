use common::v1::types::emoji::EmojiOwner;
use moka::future::Cache;
use std::collections::HashSet;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tracing::error;

use common::v1::types::misc::Color;

use common::v1::types::notifications::{Notification, NotificationReason};

use common::v1::types::util::{Diff, Time};

use common::v1::types::{
    ChannelId, ChannelPatch, Embed, EmbedCreate, EmbedId, EmbedType, Mentions, MentionsChannel,
    MentionsEmoji, MentionsRole, MentionsUser, Message, MessageCreate, MessageDefaultMarkdown,
    MessageId, MessagePatch, MessageSync, MessageType, NotificationId, Permission, RoomId,
    ThreadMembership,
};
use common::v1::types::{ThreadMemberPut, UserId};
use http::StatusCode;
use linkify::LinkFinder;
use url::Url;
use validator::Validate;

use crate::types::{DbMessageCreate, MediaLinkType, MentionsIds};
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

        // Check if a thread was created from this message
        let thread_channel_id: ChannelId = (*message.id).into();
        if let Ok(thread) = self
            .state
            .services()
            .channels
            .get(thread_channel_id, Some(user_id))
            .await
        {
            message.thread = Some(Box::new(thread));
        }

        Ok(message)
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
                                version_id: message.version_id,
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
        user_id: UserId,
        _reason: Option<String>,
        nonce: Option<String>,
        json: MessageCreate,
    ) -> Result<Message> {
        if let Some(n) = &nonce {
            self.idempotency_keys
                .try_get_with(
                    n.clone(),
                    self.create2(thread_id, user_id, _reason, nonce, json),
                )
                .await
                .map_err(|err| err.fake_clone())
        } else {
            self.create2(thread_id, user_id, _reason, nonce, json).await
        }
    }

    async fn create2(
        &self,
        thread_id: ChannelId,
        user_id: UserId,
        _reason: Option<String>,
        nonce: Option<String>,
        mut json: MessageCreate,
    ) -> Result<Message> {
        json.validate()?;
        let s = &self.state;
        let data = s.data();
        let srv = s.services();

        let user = data.user_get(user_id).await?;
        let is_webhook = user.webhook.is_some();

        let thread = srv.channels.get(thread_id, Some(user_id)).await?;

        let can_use_external_emoji = if !is_webhook {
            let perms = srv.perms.for_channel(user_id, thread_id).await?;
            perms.ensure(Permission::ViewChannel)?;
            perms.ensure(Permission::MessageCreate)?;

            if !perms.can_bypass_slowmode() {
                if let Some(message_slowmode_expire_at) = data
                    .channel_get_message_slowmode_expire_at(thread_id, user_id)
                    .await?
                {
                    if message_slowmode_expire_at > Time::now_utc() {
                        return Err(Error::BadStatic("slowmode in effect"));
                    }
                }

                if let Some(slowmode_delay) = thread.slowmode_message {
                    let next_message_time =
                        Time::now_utc() + std::time::Duration::from_secs(slowmode_delay);
                    data.channel_set_message_slowmode_expire_at(
                        thread_id,
                        user_id,
                        next_message_time,
                    )
                    .await?;
                }
            }

            if thread.archived_at.is_some() {
                srv.channels
                    .update(
                        user_id,
                        thread_id,
                        ChannelPatch {
                            archived: Some(false),
                            ..Default::default()
                        },
                        None,
                    )
                    .await?;
            }
            if !json.attachments.is_empty() {
                perms.ensure(Permission::MessageAttachments)?;
            }
            if !json.embeds.is_empty() {
                perms.ensure(Permission::MessageEmbeds)?;
            }
            if json.created_at.is_some() {
                if let Some(puppet) = user.puppet {
                    let owner_perms = srv.perms.for_channel(puppet.owner_id, thread_id).await?;
                    owner_perms.ensure(Permission::ViewChannel)?;
                    owner_perms.ensure(Permission::MemberBridge)?;
                } else {
                    return Err(Error::BadStatic("not a puppet"));
                }
            }

            perms.has(Permission::EmojiUseExternal)
        } else {
            true
        };

        // TODO: move this to validation
        if json.content.as_ref().is_none_or(|s| s.is_empty())
            && json.attachments.is_empty()
            && json.embeds.is_empty()
        {
            return Err(Error::BadStatic(
                "at least one of content, attachments, or embeds must be defined",
            ));
        }

        let attachment_ids: Vec<_> = json.attachments.iter().map(|r| r.id).collect();
        let mut all_media_ids = std::collections::HashSet::new();
        for id in &attachment_ids {
            if !all_media_ids.insert(*id) {
                return Err(Error::BadStatic("duplicate media id in request"));
            }
        }

        if !json.embeds.is_empty() {
            for embed in &json.embeds {
                if let Some(m) = &embed.media {
                    if !all_media_ids.insert(m.id) {
                        return Err(Error::BadStatic("duplicate media id in request"));
                    }
                }
                if let Some(m) = &embed.thumbnail {
                    if !all_media_ids.insert(m.id) {
                        return Err(Error::BadStatic("duplicate media id in request"));
                    }
                }
                if let Some(m) = &embed.author_avatar {
                    if !all_media_ids.insert(m.id) {
                        return Err(Error::BadStatic("duplicate media id in request"));
                    }
                }
            }
        }

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

        let mut embeds = vec![];
        if !json.embeds.is_empty() {
            let mut embed_futs = Vec::new();
            for embed_create in json.embeds.clone() {
                embed_futs.push(embed_from_create(s.clone(), embed_create, user_id));
            }
            embeds = futures_util::future::try_join_all(embed_futs).await?;
        }

        let content = json.content.clone();

        let parsed_mentions =
            mentions::parse(content.as_deref().unwrap_or_default(), &json.mentions);
        let mentions = self
            .fetch_full_mentions_from_ids(parsed_mentions, thread.room_id)
            .await?;
        if let Some(room_id) = thread.room_id {
            if let Some(c) = &mut json.content {
                *c = self
                    .enforce_emoji_use_external(&mentions, room_id, can_use_external_emoji, &c)
                    .await?;
            }
        }

        let payload = MessageType::DefaultMarkdown(MessageDefaultMarkdown {
            content: json.content,
            attachments: vec![],
            embeds: vec![],
            metadata: json.metadata,
            reply_id: json.reply_id,
            override_name: json.override_name,
        });

        let message_id = data
            .message_create(DbMessageCreate {
                channel_id: thread_id,
                attachment_ids: attachment_ids.clone(),
                author_id: user_id,
                embeds,
                message_type: payload,
                edited_at: None,
                created_at: json.created_at.map(|t| t.into()),
                mentions: mentions.clone(),
            })
            .await?;
        let message_uuid = message_id.into_inner();
        for id in &all_media_ids {
            data.media_link_insert(*id, message_uuid, MediaLinkType::Message)
                .await?;
            data.media_link_insert(*id, message_uuid, MediaLinkType::MessageVersion)
                .await?;
        }
        let mut message = self.get(thread_id, message_id, user_id).await?;

        if let Some(content) = &content {
            let mut should_embed = is_webhook;
            if !should_embed {
                if let Ok(perms) = srv.perms.for_channel(user_id, thread_id).await {
                    should_embed = perms.has(Permission::MessageEmbeds);
                }
            }

            if should_embed {
                tokio::spawn(self.handle_url_embed(message.clone(), user_id, content.clone()));
            }
        }
        s.presign_message(&mut message).await?;
        message.nonce = nonce;

        let tm = data.thread_member_get(thread_id, user_id).await;
        if tm.is_err() || tm.is_ok_and(|tm| tm.membership == ThreadMembership::Leave) {
            data.thread_member_put(thread_id, user_id, ThreadMemberPut::default())
                .await?;
            let thread_member = data.thread_member_get(thread_id, user_id).await?;
            let msg = MessageSync::ThreadMemberUpsert {
                member: thread_member,
            };
            s.broadcast_channel(thread_id, user_id, msg).await?;
        }

        let msg = MessageSync::MessageCreate {
            message: message.clone(),
        };
        srv.channels.invalidate(thread_id).await; // message count
        s.broadcast_channel(thread_id, user_id, msg).await?;

        let s_clone = self.state.clone();
        let author_id = user_id;
        let room_id = thread.room_id;
        let channel_is_thread = thread.ty.is_thread();

        tokio::spawn(async move {
            let mut notified_users = HashSet::new();

            // Direct user mentions
            for u in mentions.users {
                if u.id == author_id {
                    continue;
                }

                if channel_is_thread {
                    // Add user to thread if not already a member
                    let member = s_clone.data().thread_member_get(thread_id, u.id).await;
                    if member.is_err() || member.unwrap().membership == ThreadMembership::Leave {
                        if s_clone
                            .data()
                            .thread_member_put(thread_id, u.id, Default::default())
                            .await
                            .is_ok()
                        {
                            if let Ok(thread_member) =
                                s_clone.data().thread_member_get(thread_id, u.id).await
                            {
                                let msg = MessageSync::ThreadMemberUpsert {
                                    member: thread_member,
                                };
                                if let Err(e) =
                                    s_clone.broadcast_channel(thread_id, author_id, msg).await
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
                        .unread_increment_mentions(
                            u.id,
                            thread_id,
                            message_id,
                            message.version_id,
                            1,
                        )
                        .await
                    {
                        error!("Failed to increment mention count for user {}: {}", u.id, e);
                    }
                    let notification = Notification {
                        id: NotificationId::new(),
                        channel_id: thread_id,
                        message_id,
                        reason: NotificationReason::Mention,
                        added_at: Time::now_utc(),
                        read_at: None,
                    };
                    if let Err(e) = s_clone.data().notification_add(u.id, notification).await {
                        error!(
                            "Failed to add mention notification for user {}: {}",
                            u.id, e
                        );
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
                                        thread_id,
                                        member.user_id,
                                        Default::default(),
                                    )
                                    .await
                                {
                                    error!(
                                        "Failed to add mentioned role member {} to thread {}: {}",
                                        member.user_id, thread_id, e
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
                                        thread_id,
                                        message_id,
                                        message.version_id,
                                        1,
                                    )
                                    .await
                                {
                                    error!(
                                        "Failed to increment mention count for user {}: {}",
                                        member.user_id, e
                                    );
                                }
                                let notification = Notification {
                                    id: NotificationId::new(),
                                    channel_id: thread_id,
                                    message_id,
                                    reason: NotificationReason::Mention,
                                    added_at: Time::now_utc(),
                                    read_at: None,
                                };
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

            // @everyone mentions
            if mentions.everyone {
                let mut users_to_notify = Vec::new();
                if channel_is_thread {
                    if let Ok(members) = s_clone.data().thread_member_list_all(thread_id).await {
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
                                user_id,
                                thread_id,
                                message_id,
                                message.version_id,
                                1,
                            )
                            .await
                        {
                            error!(
                                "Failed to increment mention count for user {}: {}",
                                user_id, e
                            );
                        }
                        let notification = Notification {
                            id: NotificationId::new(),
                            channel_id: thread_id,
                            message_id,
                            reason: NotificationReason::MentionBulk,
                            added_at: Time::now_utc(),
                            read_at: None,
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

        Ok(message)
    }

    // TODO: refactor create and edit together
    // FIXME: webhook permisison checks
    // FIXME: use external emoji permission checks
    pub async fn edit(
        &self,
        thread_id: ChannelId,
        message_id: MessageId,
        user_id: UserId,
        _reason: Option<String>,
        json: MessagePatch,
    ) -> Result<(StatusCode, Message)> {
        let s = &self.state;
        json.validate()?;
        let data = s.data();
        let srv = s.services();
        let user = srv.users.get(user_id, None).await?;
        let is_webhook = user.webhook.is_some();

        let perms = if is_webhook {
            None
        } else {
            Some(s.services().perms.for_channel(user_id, thread_id).await?)
        };

        if let Some(perms) = &perms {
            perms.ensure(Permission::ViewChannel)?;
        }

        let message = match self.get(thread_id, message_id, user_id).await {
            Ok(m) => m,
            Err(e) => {
                if is_webhook {
                    return Err(Error::NotFound);
                }
                return Err(e);
            }
        };

        if !message.message_type.is_editable() {
            return Err(Error::BadStatic("cant edit that message"));
        }
        if message.author_id != user_id {
            if is_webhook {
                return Err(Error::NotFound);
            }
            return Err(Error::BadStatic("cant edit other user's message"));
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
            if json.attachments.as_ref().is_none_or(|a| !a.is_empty()) {
                perms.ensure(Permission::MessageAttachments)?;
            }
            if json.embeds.as_ref().is_none_or(|a| !a.is_empty()) {
                perms.ensure(Permission::MessageEmbeds)?;
            }
        }

        if json.edited_at.is_some() {
            if is_webhook {
                return Err(Error::BadStatic("webhook cannot set edited_at"));
            }
            let usr = data.user_get(user_id).await?;
            if let Some(puppet) = usr.puppet {
                let owner_perms = srv.perms.for_channel(puppet.owner_id, thread_id).await?;
                owner_perms.ensure(Permission::ViewChannel)?;
                owner_perms.ensure(Permission::MemberBridge)?;
            } else {
                return Err(Error::BadStatic("not a puppet"));
            }
        }

        if !json.changes(&message) {
            return Ok((StatusCode::NOT_MODIFIED, message));
        }
        let attachment_ids: Vec<_> = json
            .attachments
            .map(|ats| ats.into_iter().map(|r| r.id).collect())
            .unwrap_or_else(|| match &message.message_type {
                MessageType::DefaultMarkdown(msg) => {
                    msg.attachments.iter().map(|media| media.id).collect()
                }
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
                embed_futs.push(embed_from_create(s.clone(), embed_create, user_id));
            }
            embeds = futures_util::future::try_join_all(embed_futs).await?;
        }

        let (content, payload) = match message.message_type.clone() {
            MessageType::DefaultMarkdown(msg) => {
                let content = json.content.unwrap_or(msg.content);
                Result::Ok((
                    content.clone(),
                    MessageType::DefaultMarkdown(MessageDefaultMarkdown {
                        content,
                        attachments: vec![],
                        embeds: embeds.clone(),
                        metadata: json.metadata.unwrap_or(msg.metadata),
                        reply_id: json.reply_id.unwrap_or(msg.reply_id),
                        override_name: json.override_name.unwrap_or(msg.override_name),
                    }),
                ))
            }
            _ => return Err(Error::Unimplemented),
        }?;
        let version_id = data
            .message_update(
                thread_id,
                message_id,
                DbMessageCreate {
                    channel_id: thread_id,
                    attachment_ids: attachment_ids.clone(),
                    author_id: user_id,
                    embeds,
                    message_type: payload,
                    edited_at: json.edited_at.map(|t| t.into()),
                    created_at: message.created_at.map(|t| t.into()),
                    mentions: message.mentions,
                },
            )
            .await?;

        for id in &attachment_ids {
            data.media_link_insert(*id, *version_id, MediaLinkType::MessageVersion)
                .await?;
            data.media_link_insert(*id, *message_id, MediaLinkType::Message)
                .await?;
        }

        let mut message = data
            .message_version_get(thread_id, version_id, user_id)
            .await?;

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
        Ok((StatusCode::CREATED, message))
    }

    // TODO: batch fetching
    pub async fn fetch_full_mentions_from_ids(
        &self,
        mentions_ids: MentionsIds,
        room_id: Option<RoomId>,
    ) -> Result<Mentions> {
        let srv = self.state.services();
        let data = self.state.data();

        let mut mentions = Mentions {
            users: vec![],
            roles: vec![],
            channels: vec![],
            emojis: vec![],
            everyone: mentions_ids.everyone,
        };

        for user_id in mentions_ids.users {
            let user = srv.users.get(user_id, None).await?;
            let room_member = if let Some(room_id) = room_id {
                data.room_member_get(room_id, user_id).await.ok()
            } else {
                None
            };

            let resolved_name = if let Some(room_member) = room_member {
                room_member
                    .override_name
                    .unwrap_or_else(|| user.name.clone())
            } else {
                user.name.clone()
            };

            mentions.users.push(MentionsUser {
                id: user_id,
                resolved_name,
            });
        }

        if let Some(room_id) = room_id {
            for role_id in mentions_ids.roles {
                let _role = data.role_select(room_id, role_id).await?;
                mentions.roles.push(MentionsRole { id: role_id });
            }
        }

        for channel_id in mentions_ids.channels {
            let channel = srv.channels.get(channel_id, None).await?;
            mentions.channels.push(MentionsChannel {
                id: channel_id,
                room_id: channel.room_id,
                ty: channel.ty,
                name: channel.name,
            });
        }

        for emoji_id in mentions_ids.emojis {
            let emoji = data.emoji_get(emoji_id).await?;
            mentions.emojis.push(MentionsEmoji {
                id: emoji_id,
                name: emoji.name,
                animated: emoji.animated,
            });
        }

        Ok(mentions)
    }

    // TODO(#833): enforce EmojiUseExternal permission
    async fn enforce_emoji_use_external(
        &self,
        m: &Mentions,
        room_id: RoomId,
        allow: bool,
        content: &str,
    ) -> Result<String> {
        let data = self.state.data();
        let mut allowed_emoji = vec![];

        for i in &m.emojis {
            let emoji = data.emoji_get(i.id).await?;
            let is_room_emoji = emoji.owner == Some(EmojiOwner::Room { room_id });
            if is_room_emoji || allow {
                allowed_emoji.push(emoji.id);
            }
        }

        Ok(mentions::strip_emoji(content, &allowed_emoji))
    }

    // TODO: move data stuff to this service
    // pub async fn list(
    //     &self,
    //     channel_id: ChannelId,
    //     user_id: UserId,
    //     pagination: PaginationQuery<MessageId>,
    // ) -> Result<PaginationResponse<Message>> {
    //     todo!()
    // }

    // pub fn list_deleted(&self) {}
    // pub fn list_removed(&self) {}
    // pub fn list_context(&self) {}
    // pub fn list_versions(&self) {}
    // pub fn get_version(&self) {}
    // pub fn list_replies(&self) {}
    // pub fn list_pins(&self) {}
    // pub fn list_activity(&self) {}
}

// this should probably be moved somewhere else
async fn embed_from_create(
    s: Arc<ServerStateInner>,
    value: EmbedCreate,
    user_id: UserId,
) -> Result<Embed> {
    let media = if let Some(media_ref) = value.media {
        let media = s.data().media_select(media_ref.id).await?;
        if media.user_id != user_id {
            return Err(Error::MissingPermissions);
        }
        Some(media)
    } else {
        None
    };
    let thumbnail = if let Some(media_ref) = value.thumbnail {
        let media = s.data().media_select(media_ref.id).await?;
        if media.user_id != user_id {
            return Err(Error::MissingPermissions);
        }
        Some(media)
    } else {
        None
    };
    let author_avatar = if let Some(media_ref) = value.author_avatar {
        let media = s.data().media_select(media_ref.id).await?;
        if media.user_id != user_id {
            return Err(Error::MissingPermissions);
        }
        Some(media)
    } else {
        None
    };

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
