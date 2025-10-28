use moka::future::Cache;
use std::collections::HashSet;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tracing::error;

use common::v1::types::misc::Color;
use common::v1::types::util::Diff;
use common::v1::types::{
    ChannelId, ChannelPatch, Embed, Message, MessageCreate, MessageDefaultMarkdown, MessageId,
    MessagePatch, MessageSync, MessageType, Permission, ThreadMembership,
};
use common::v1::types::{ThreadMemberPut, UserId};
use http::StatusCode;
use linkify::LinkFinder;
use url::Url;
use validator::Validate;

use crate::types::{DbMessageCreate, MediaLinkType};
use crate::{Error, Result, ServerStateInner};

pub struct ServiceMessages {
    state: Arc<ServerStateInner>,
    pub idempotency_keys: Cache<String, Message>,
}

impl ServiceMessages {
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
        json: MessageCreate,
    ) -> Result<Message> {
        json.validate()?;
        let s = &self.state;
        let data = s.data();
        let srv = s.services();

        let user = data.user_get(user_id).await?;
        let is_webhook = user.webhook.is_some();

        let thread = srv.channels.get(thread_id, Some(user_id)).await?;

        if !is_webhook {
            let perms = srv.perms.for_channel(user_id, thread_id).await?;
            perms.ensure(Permission::ViewChannel)?;
            perms.ensure(Permission::MessageCreate)?;

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
        }

        // TODO: move this to validation
        if json.content.as_ref().is_none_or(|s| s.is_empty())
            && json.attachments.is_empty()
            && json.embeds.is_empty()
        {
            return Err(Error::BadStatic(
                "at least one of content, attachments, or embeds must be defined",
            ));
        }

        let attachment_ids: Vec<_> = json.attachments.into_iter().map(|r| r.id).collect();
        for id in &attachment_ids {
            data.media_select(*id).await?;
            let existing = data.media_link_select(*id).await?;
            if !existing.is_empty() {
                return Err(Error::BadStatic("cant reuse media"));
            }
        }
        let content = json.content.clone();
        let parsed_mentions = dbg!(mentions::parse(
            content.as_deref().unwrap_or_default(),
            &json.mentions
        ));
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
                embeds: json
                    .embeds
                    .clone()
                    .into_iter()
                    .map(embed_from_create)
                    .collect(),
                message_type: payload,
                edited_at: None,
                created_at: json.created_at.map(|t| t.into()),
                mentions: parsed_mentions.clone(),
            })
            .await?;
        let message_uuid = message_id.into_inner();
        for id in &attachment_ids {
            data.media_link_insert(*id, message_uuid, MediaLinkType::Message)
                .await?;
            data.media_link_insert(*id, message_uuid, MediaLinkType::MessageVersion)
                .await?;
        }
        let mut message = data.message_get(thread_id, message_id, user_id).await?;

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
            for user_id in parsed_mentions.users {
                if user_id == author_id {
                    continue;
                }

                if channel_is_thread {
                    // Add user to thread if not already a member
                    let member = s_clone.data().thread_member_get(thread_id, user_id).await;
                    if member.is_err() || member.unwrap().membership == ThreadMembership::Leave {
                        if s_clone
                            .data()
                            .thread_member_put(thread_id, user_id, Default::default())
                            .await
                            .is_ok()
                        {
                            if let Ok(thread_member) =
                                s_clone.data().thread_member_get(thread_id, user_id).await
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
                }
            }

            // Role mentions
            if let Some(_room_id) = room_id {
                for role_id in parsed_mentions.roles {
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
                            }
                        }
                    }
                }
            }

            // @everyone mentions
            if parsed_mentions.everyone {
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
                    }
                }
            }
        });

        Ok(message)
    }

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
        let perms = s.services().perms.for_channel(user_id, thread_id).await?;
        perms.ensure(Permission::ViewChannel)?;
        let message = data.message_get(thread_id, message_id, user_id).await?;
        if !message.message_type.is_editable() {
            return Err(Error::BadStatic("cant edit that message"));
        }
        if message.author_id != user_id {
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
        if json.attachments.as_ref().is_none_or(|a| !a.is_empty()) {
            perms.ensure(Permission::MessageAttachments)?;
        }
        if json.embeds.as_ref().is_none_or(|a| !a.is_empty()) {
            perms.ensure(Permission::MessageEmbeds)?;
        }
        if json.edited_at.is_some() {
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
        let (content, payload) = match message.message_type.clone() {
            MessageType::DefaultMarkdown(msg) => {
                let content = json.content.unwrap_or(msg.content);
                Result::Ok((
                    content.clone(),
                    MessageType::DefaultMarkdown(MessageDefaultMarkdown {
                        content,
                        attachments: vec![],
                        embeds: json
                            .embeds
                            .clone()
                            .unwrap_or_default()
                            .into_iter()
                            .map(embed_from_create)
                            .collect(),
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
                    embeds: json
                        .embeds
                        .clone()
                        .unwrap_or_default()
                        .into_iter()
                        .map(embed_from_create)
                        .collect(),
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
            if perms.has(Permission::MessageEmbeds) {
                tokio::spawn(self.handle_url_embed(message.clone(), user_id, content.clone()));
            }
        }

        if let Some(embeds) = json.embeds {
            match &mut message.message_type {
                MessageType::DefaultMarkdown(m) => {
                    m.embeds = embeds.into_iter().map(embed_from_create).collect()
                }
                _ => {}
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
}

fn embed_from_create(value: common::v1::types::EmbedCreate) -> Embed {
    Embed {
        id: common::v1::types::EmbedId::new(),
        ty: common::v1::types::EmbedType::Custom,
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
        media: None,
        thumbnail: None,
        author_name: value.author_name,
        author_url: value.author_url,
        author_avatar: None,
        site_name: None,
        site_avatar: None,
    }
}

pub mod mentions {
    use common::v1::types::{EmojiId, Mentions, ParseMentions, RoleId, UserId};
    use once_cell::sync::Lazy;
    use regex::Regex;
    use std::collections::HashSet;
    use uuid::Uuid;

    static USER_MENTION_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r"<@([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})>",
        )
        .unwrap()
    });
    static ROLE_MENTION_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r"<@&([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})>",
        )
        .unwrap()
    });
    static CHANNEL_MENTION_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r"<#([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})>",
        )
        .unwrap()
    });
    static EMOJI_MENTION_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r"<a?:\w+:([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-?[0-9a-fA-F]{4}-?[0-9a-fA-F]{4}-?[0-9a-fA-F]{12})>",
        )
        .unwrap()
    });
    static EVERYONE_MENTION_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"@everyone").unwrap());

    pub fn parse(content: &str, options: &ParseMentions) -> Mentions {
        let users = options
            .users
            .as_ref()
            .map(|allowed_users| {
                USER_MENTION_RE
                    .captures_iter(content)
                    .filter_map(|cap| {
                        let id = Uuid::parse_str(&cap[1]).ok()?.into();
                        if allowed_users.contains(&id) {
                            Some(id)
                        } else {
                            None
                        }
                    })
                    .collect::<HashSet<UserId>>()
                    .into_iter()
                    .collect()
            })
            .unwrap_or_else(|| {
                USER_MENTION_RE
                    .captures_iter(content)
                    .filter_map(|cap| Uuid::parse_str(&cap[1]).ok().map(Into::into))
                    .collect::<HashSet<UserId>>()
                    .into_iter()
                    .collect()
            });

        let roles = options
            .roles
            .as_ref()
            .map(|allowed_roles| {
                ROLE_MENTION_RE
                    .captures_iter(content)
                    .filter_map(|cap| {
                        let id = Uuid::parse_str(&cap[1]).ok()?.into();
                        if allowed_roles.contains(&id) {
                            Some(id)
                        } else {
                            None
                        }
                    })
                    .collect::<HashSet<RoleId>>()
                    .into_iter()
                    .collect()
            })
            .unwrap_or_else(|| {
                ROLE_MENTION_RE
                    .captures_iter(content)
                    .filter_map(|cap| Uuid::parse_str(&cap[1]).ok().map(Into::into))
                    .collect::<HashSet<RoleId>>()
                    .into_iter()
                    .collect()
            });

        let channels = CHANNEL_MENTION_RE
            .captures_iter(content)
            .filter_map(|cap| Uuid::parse_str(&cap[1]).ok().map(Into::into))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        let emojis = EMOJI_MENTION_RE
            .captures_iter(content)
            .filter_map(|cap| Uuid::parse_str(&cap[1]).ok().map(Into::into))
            .collect::<HashSet<EmojiId>>()
            .into_iter()
            .collect();

        let everyone = options.everyone && EVERYONE_MENTION_RE.is_match(content);

        Mentions {
            users,
            roles,
            threads: channels,
            emojis,
            everyone,
        }
    }
}
