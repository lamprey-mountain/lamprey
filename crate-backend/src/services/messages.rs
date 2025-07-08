use std::sync::Arc;

use common::v1::types::misc::Color;
use common::v1::types::reaction::ReactionCounts;
use common::v1::types::util::Diff;
use common::v1::types::UserId;
use common::v1::types::{
    Embed, Interactions, Message, MessageCreate, MessageDefaultMarkdown, MessageDefaultTagged,
    MessageId, MessagePatch, MessageSync, MessageType, Permission, ThreadId, ThreadMembership,
};
use http::StatusCode;
use linkify::LinkFinder;
use url::Url;
use validator::Validate;

use crate::types::{DbMessageCreate, MediaLinkType};
use crate::{Error, Result, ServerStateInner};

pub struct ServiceMessages {
    state: Arc<ServerStateInner>,
}

impl ServiceMessages {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self { state }
    }

    pub async fn create(
        &self,
        thread_id: ThreadId,
        user_id: UserId,
        reason: Option<String>,
        nonce: Option<String>,
        json: MessageCreate,
    ) -> Result<Message> {
        json.validate()?;
        let s = &self.state;
        let data = s.data();
        let srv = s.services();
        let perms = srv.perms.for_thread(user_id, thread_id).await?;
        perms.ensure_view()?;
        perms.ensure(Permission::MessageCreate)?;
        if !json.attachments.is_empty() {
            perms.ensure(Permission::MessageAttachments)?;
        }
        if !json.embeds.is_empty() {
            perms.ensure(Permission::MessageEmbeds)?;
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
            reactions: ReactionCounts::default(),
        });
        let message_id = data
            .message_create(DbMessageCreate {
                thread_id,
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
            })
            .await?;
        let message_uuid = message_id.into_inner();
        for id in &attachment_ids {
            data.media_link_insert(*id, message_uuid, MediaLinkType::Message)
                .await?;
            data.media_link_insert(*id, message_uuid, MediaLinkType::MessageVersion)
                .await?;
        }
        let mut message = data.message_get(thread_id, message_id).await?;

        if let Some(content) = &content {
            for link in LinkFinder::new().links(content) {
                if let Some(url) = link.as_str().parse::<Url>().ok() {
                    let s = s.clone();
                    let srv = srv.clone();
                    let data = s.data();
                    tokio::spawn(async move {
                        let message = data.message_get(thread_id, message_id).await?;
                        let mut new_message_type = message.message_type.clone();
                        let (embeds, attachments) = match &mut new_message_type {
                            MessageType::DefaultMarkdown(m) => {
                                if m.embeds.iter().any(|e| e.url.as_ref() == Some(&url)) {
                                    return Ok(());
                                }
                                let embed = srv.embed.generate(user_id, url.clone()).await?;
                                m.embeds.push(embed);
                                (
                                    m.embeds.clone(),
                                    m.attachments.iter().map(|a| a.id).collect(),
                                )
                            }
                            MessageType::DefaultTagged(m) => {
                                if m.embeds.iter().any(|e| e.url.as_ref() == Some(&url)) {
                                    return Ok(());
                                }
                                let embed = srv.embed.generate(user_id, url.clone()).await?;
                                m.embeds.push(embed);
                                (
                                    m.embeds.clone(),
                                    m.attachments.iter().map(|a| a.id).collect(),
                                )
                            }
                            _ => return Ok(()),
                        };

                        data.message_update(
                            thread_id,
                            message_id,
                            DbMessageCreate {
                                thread_id,
                                attachment_ids: attachments,
                                author_id: message.author_id,
                                embeds,
                                message_type: new_message_type,
                                edited_at: None,
                            },
                        )
                        .await?;

                        let mut message = data.message_get(thread_id, message_id).await?;
                        s.presign_message(&mut message).await?;
                        s.broadcast_thread(
                            thread_id,
                            user_id,
                            None,
                            MessageSync::MessageUpdate { message },
                        )
                        .await?;
                        Result::Ok(())
                    });
                }
            }
        }
        s.presign_message(&mut message).await?;
        message.nonce = nonce.or(json.nonce);
        data.thread_member_put(
            thread_id,
            user_id,
            ThreadMembership::Join {
                override_name: None,
                override_description: None,
            },
        )
        .await?;
        let msg = MessageSync::MessageCreate {
            message: message.clone(),
        };
        srv.threads.invalidate(thread_id); // message count
        s.broadcast_thread(thread_id, user_id, reason, msg).await?;
        Ok(message)
    }

    pub async fn edit(
        &self,
        thread_id: ThreadId,
        message_id: MessageId,
        user_id: UserId,
        reason: Option<String>,
        json: MessagePatch,
    ) -> Result<(StatusCode, Message)> {
        let s = &self.state;
        json.validate()?;
        let data = s.data();
        let mut perms = s.services().perms.for_thread(user_id, thread_id).await?;
        perms.ensure_view()?;
        let message = data.message_get(thread_id, message_id).await?;
        if !message.message_type.is_editable() {
            return Err(Error::BadStatic("cant edit that message"));
        }
        if message.author_id == user_id {
            perms.add(Permission::MessageEdit);
        }
        perms.ensure(Permission::MessageEdit)?;
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
                        reactions: ReactionCounts::default(),
                    }),
                ))
            }
            MessageType::DefaultTagged(msg) => {
                let content = json.content.unwrap_or(msg.content);
                Result::Ok((
                    content.clone(),
                    MessageType::DefaultTagged(MessageDefaultTagged {
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
                        reactions: ReactionCounts(vec![]),
                        interactions: Interactions::default(),
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
                    thread_id,
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
                    edited_at: None,
                },
            )
            .await?;

        let version_uuid = version_id.into_inner();
        for id in &attachment_ids {
            data.media_link_insert(*id, version_uuid, MediaLinkType::MessageVersion)
                .await?;
        }

        if let Some(content) = &content {
            let srv = s.services();
            for link in LinkFinder::new().links(content).into_iter() {
                if let Some(url) = link.as_str().parse::<Url>().ok() {
                    let s = s.clone();
                    let srv = srv.clone();
                    let data = s.data();
                    tokio::spawn(async move {
                        let message = data.message_get(thread_id, message_id).await?;
                        let mut new_message_type = message.message_type.clone();
                        let (embeds, attachments) = match &mut new_message_type {
                            MessageType::DefaultMarkdown(m) => {
                                if m.embeds.iter().any(|e| e.url.as_ref() == Some(&url)) {
                                    return Ok(());
                                }
                                let embed = srv.embed.generate(user_id, url.clone()).await?;
                                m.embeds.push(embed);
                                (
                                    m.embeds.clone(),
                                    m.attachments.iter().map(|a| a.id).collect(),
                                )
                            }
                            MessageType::DefaultTagged(m) => {
                                if m.embeds.iter().any(|e| e.url.as_ref() == Some(&url)) {
                                    return Ok(());
                                }
                                let embed = srv.embed.generate(user_id, url.clone()).await?;
                                m.embeds.push(embed);
                                (
                                    m.embeds.clone(),
                                    m.attachments.iter().map(|a| a.id).collect(),
                                )
                            }
                            _ => return Ok(()),
                        };

                        data.message_update(
                            thread_id,
                            message_id,
                            DbMessageCreate {
                                thread_id,
                                attachment_ids: attachments,
                                author_id: message.author_id,
                                embeds,
                                message_type: new_message_type,
                                edited_at: None,
                            },
                        )
                        .await?;

                        let mut message = data.message_get(thread_id, message_id).await?;
                        s.presign_message(&mut message).await?;
                        s.broadcast_thread(
                            thread_id,
                            user_id,
                            None,
                            MessageSync::MessageUpdate { message },
                        )
                        .await?;
                        Result::Ok(())
                    });
                }
            }
        }

        let mut message = data.message_version_get(thread_id, version_id).await?;

        if let Some(embeds) = json.embeds {
            match &mut message.message_type {
                MessageType::DefaultMarkdown(m) => {
                    m.embeds = embeds.into_iter().map(embed_from_create).collect()
                }
                MessageType::DefaultTagged(m) => {
                    m.embeds = embeds.into_iter().map(embed_from_create).collect()
                }
                _ => {}
            }
        }

        s.presign_message(&mut message).await?;
        s.broadcast_thread(
            thread_id,
            user_id,
            reason,
            MessageSync::MessageUpdate {
                message: message.clone(),
            },
        )
        .await?;
        s.services().threads.invalidate(thread_id); // last version id
        Ok((StatusCode::CREATED, message))
    }
}

fn embed_from_create(value: common::v1::types::EmbedCreate) -> Embed {
    Embed {
        id: common::v1::types::EmbedId::new(),
        url: value.url,
        canonical_url: None,
        title: value.title,
        description: value.description,
        color: value
            .color
            .map(|s| csscolorparser::parse(&s))
            .transpose()
            .expect("invalid color")
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
