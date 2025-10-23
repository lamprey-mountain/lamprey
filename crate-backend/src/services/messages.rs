use moka::future::Cache;
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
        let perms = srv.perms.for_channel(user_id, thread_id).await?;
        perms.ensure(Permission::ViewChannel)?;
        perms.ensure(Permission::MessageCreate)?;

        let thread = srv.channels.get(thread_id, Some(user_id)).await?;
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
            let usr = data.user_get(user_id).await?;
            if let Some(puppet) = usr.puppet {
                let owner_perms = srv.perms.for_channel(puppet.owner_id, thread_id).await?;
                owner_perms.ensure(Permission::ViewChannel)?;
                owner_perms.ensure(Permission::MemberBridge)?;
            } else {
                return Err(Error::BadStatic("not a puppet"));
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
                mentions: json.mentions,
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
            if perms.has(Permission::MessageEmbeds) {
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
