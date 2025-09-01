use std::str::FromStr;
use std::sync::Arc;

use crate::common::Globals;
use crate::data::AttachmentMetadata;
use crate::data::Data;
use crate::data::MessageMetadata;
use crate::data::PortalConfig;
use crate::data::Puppet;
use crate::discord::DiscordMessage;
use anyhow::Result;
use common::v1::types::media::MediaRef;
use common::v1::types::EmbedCreate;
use common::v1::types::RoomId;
use common::v1::types::{self, Message, MessageId, ThreadId};
use reqwest::Url;
use serenity::all::CreateAllowedMentions;
use serenity::all::CreateAttachment;
use serenity::all::CreateEmbed;
use serenity::all::EditAttachments;
use serenity::all::EditWebhookMessage;
use serenity::all::Mentionable;
use serenity::all::{
    ChannelId as DcChannelId, Message as DcMessage, MessageId as DcMessageId,
    MessageType as DcMessageType, MessageUpdateEvent as DcMessageUpdate, Reaction as DcReaction,
};
use serenity::all::{ExecuteWebhook, MessageReferenceKind};
use std::fmt::Debug;
use time::OffsetDateTime;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info};

pub struct Portal {
    globals: Arc<Globals>,
    recv: mpsc::UnboundedReceiver<PortalMessage>,
    config: PortalConfig,
}

impl Debug for Portal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Portal ({:?})", self.config)
    }
}

/// portal actor message
#[derive(Debug)]
pub enum PortalMessage {
    LampreyMessageCreate {
        message: Message,
    },
    LampreyMessageUpdate {
        message: Message,
    },
    LampreyMessageDelete {
        message_id: MessageId,
    },
    DiscordMessageCreate {
        message: DcMessage,
    },
    DiscordMessageUpdate {
        update: DcMessageUpdate,
    },
    DiscordMessageDelete {
        message_id: DcMessageId,
    },
    DiscordReactionAdd {
        add_reaction: DcReaction,
    },
    DiscordReactionRemove {
        removed_reaction: DcReaction,
    },
    DiscordTyping {
        user_id: serenity::model::id::UserId,
    },
}

impl Portal {
    pub fn summon(
        globals: Arc<Globals>,
        config: PortalConfig,
    ) -> mpsc::UnboundedSender<PortalMessage> {
        let (send, recv) = mpsc::unbounded_channel();
        let portal = Self {
            globals,
            recv,
            config,
        };
        tokio::spawn(portal.activate());
        send
    }

    pub fn channel_id(&self) -> DcChannelId {
        self.config.discord_channel_id
    }

    pub fn thread_id(&self) -> ThreadId {
        self.config.lamprey_thread_id
    }

    pub fn room_id(&self) -> RoomId {
        self.config.lamprey_room_id
    }

    async fn activate(mut self) {
        while let Some(msg) = self.recv.recv().await {
            if let Err(err) = self.handle(msg).await {
                error!("{err}")
            }
        }
    }

    async fn handle(&mut self, msg: PortalMessage) -> Result<()> {
        match msg {
            PortalMessage::LampreyMessageCreate { message } => {
                self.handle_lamprey_message_create(message).await?;
            }
            PortalMessage::LampreyMessageUpdate { message } => {
                self.handle_lamprey_message_create(message).await?;
            }
            PortalMessage::LampreyMessageDelete { message_id } => {
                self.handle_lamprey_message_delete(message_id).await?;
            }
            PortalMessage::DiscordMessageCreate { message } => {
                self.handle_discord_message_create(message).await?;
            }
            PortalMessage::DiscordMessageUpdate { update } => {
                self.handle_discord_message_update(update).await?;
            }
            PortalMessage::DiscordMessageDelete { message_id } => {
                self.handle_discord_message_delete(message_id).await?;
            }
            PortalMessage::DiscordReactionAdd { add_reaction } => {
                self.handle_discord_reaction_add(add_reaction).await?;
            }
            PortalMessage::DiscordReactionRemove { removed_reaction } => {
                self.handle_discord_reaction_remove(removed_reaction)
                    .await?;
            }
            PortalMessage::DiscordTyping { user_id } => {
                self.handle_discord_typing(user_id).await?;
            }
        }
        Ok(())
    }

    async fn handle_lamprey_message_create(&mut self, message: Message) -> Result<()> {
        let ly = self.globals.lamprey_handle().await?;
        let user = ly.user_fetch(message.author_id).await?;
        if user.puppet.is_some() {
            debug!("not bridging message from puppet");
            return Ok(());
        }

        let existing = self.globals.get_message(message.id).await?;
        let msg_inner = match message.message_type {
            types::MessageType::DefaultMarkdown(m) => m,
            _ => {
                debug!("unknown lamprey message type");
                return Ok(());
            }
        };

        let reply_ids = if let Some(reply_id) = msg_inner.reply_id {
            self.globals
                .get_message(reply_id)
                .await?
                .map(|i| (i.discord_id, i.chat_id))
        } else {
            None
        };
        let mut embeds = vec![];
        let mut content = msg_inner.content.to_owned().unwrap_or_else(|| {
            if msg_inner.attachments.is_empty() && msg_inner.embeds.is_empty() {
                "(no content?)".to_owned()
            } else {
                "".to_owned()
            }
        });
        if let Some(reply_ids) = reply_ids {
            let (discord_id, _chat_id) = reply_ids;
            let (send, recv) = oneshot::channel();
            self.globals
                .dc_chan
                .send(DiscordMessage::MessageGet {
                    channel_id: self.channel_id(),
                    message_id: discord_id,
                    response: send,
                })
                .await?;
            let reply = recv.await?;
            let reply_content = if !reply.content.is_empty() {
                reply.content.to_owned()
            } else if !reply.attachments.is_empty() {
                let names: Vec<_> = reply
                    .attachments
                    .iter()
                    .map(|a| a.filename.to_owned())
                    .collect();
                format!(
                    "{} attachment(s): {}",
                    reply.attachments.len(),
                    names.join(", ")
                )
            } else if !reply.embeds.is_empty() {
                format!("{} embed(s)", reply.embeds.len())
            } else {
                "(no content?)".to_owned()
            };
            let description = format!(
                "**[replying to](https://canary.discord.com/channels/{}/{}/{})**\n{}",
                self.config.discord_guild_id,
                self.channel_id(),
                discord_id,
                reply_content,
            );
            content = format!("{} ਵਿਚ{}", reply.author.mention(), content);
            embeds.push(CreateEmbed::new().description(description));

            if let Some(att) = reply.attachments.first() {
                embeds.push(CreateEmbed::new().image(&att.url));
            }
        }
        let (send, recv) = tokio::sync::oneshot::channel();
        if let Some(edit) = existing {
            let mut files = EditAttachments::new();
            for media in &msg_inner.attachments {
                let existing = self.globals.get_attachment(media.id.to_owned()).await?;
                if let Some(existing) = existing {
                    files = files.keep(existing.discord_id);
                } else {
                    let url = format!(
                        "{}/media/{}",
                        self.globals
                            .config
                            .lamprey_cdn_url
                            .as_deref()
                            .unwrap_or("https://chat-cdn.celery.eu.org"),
                        media.id
                    );
                    let bytes = reqwest::get(url).await?.error_for_status()?.bytes().await?;
                    files = files.add(CreateAttachment::bytes(bytes, media.filename.to_owned()));
                }
            }
            // let files = files.into_iter().map(|i| EditAttachments::new().add()).collect();
            let mut payload = EditWebhookMessage::new()
                .content(content)
                .allowed_mentions(
                    CreateAllowedMentions::new()
                        .everyone(false)
                        .all_roles(false)
                        .all_users(true),
                )
                .embeds(embeds)
                .attachments(files);
            if let Some(dc_tid) = self.config.discord_thread_id {
                payload = payload.in_thread(dc_tid);
            }
            self.globals
                .dc_chan
                .send(DiscordMessage::WebhookMessageEdit {
                    url: self.config.discord_webhook.clone(),
                    payload,
                    response: send,
                    message_id: edit.discord_id,
                })
                .await?;
        } else {
            let mut files = vec![];
            for media in &msg_inner.attachments {
                let url = format!(
                    "{}/media/{}",
                    self.globals
                        .config
                        .lamprey_cdn_url
                        .as_deref()
                        .unwrap_or("https://chat-cdn.celery.eu.org"),
                    media.id
                );
                let bytes = reqwest::get(url).await?.error_for_status()?.bytes().await?;
                files.push(CreateAttachment::bytes(bytes, media.filename.to_owned()));
            }
            let user = ly.user_fetch(message.author_id).await?;
            let mut payload = ExecuteWebhook::new()
                .username(msg_inner.override_name.unwrap_or(user.name))
                .avatar_url("")
                .content(content)
                .allowed_mentions(
                    CreateAllowedMentions::new()
                        .everyone(false)
                        .all_roles(false)
                        .all_users(true),
                )
                .add_files(files)
                .embeds(embeds);
            if let Some(dc_tid) = self.config.discord_thread_id {
                payload = payload.in_thread(dc_tid);
            }
            if let Some(media_id) = user.avatar {
                let url = format!(
                    "{}/thumb/{}",
                    self.globals
                        .config
                        .lamprey_cdn_url
                        .as_deref()
                        .unwrap_or("https://chat-cdn.celery.eu.org"),
                    media_id
                );
                payload = payload.avatar_url(url);
            };
            self.globals
                .dc_chan
                .send(DiscordMessage::WebhookExecute {
                    url: self.config.discord_webhook.clone(),
                    payload,
                    response: send,
                })
                .await?;
        }
        let res = recv.await?;
        self.globals
            .insert_message(MessageMetadata {
                chat_id: message.id,
                chat_thread_id: message.thread_id,
                discord_id: res.id,
                discord_channel_id: res.channel_id,
            })
            .await?;

        for (att, media) in res.attachments.iter().zip(msg_inner.attachments) {
            self.globals
                .insert_attachment(AttachmentMetadata {
                    chat_id: media.id,
                    discord_id: att.id,
                })
                .await?;
        }

        Ok(())
    }

    async fn handle_lamprey_message_delete(&mut self, message_id: MessageId) -> Result<()> {
        let Some(existing) = self.globals.get_message(message_id).await? else {
            debug!("message doesnt exist or is already deleted");
            return Ok(());
        };

        self.globals.delete_message(message_id).await?;
        let (send, recv) = oneshot::channel();
        self.globals
            .dc_chan
            .send(DiscordMessage::WebhookMessageDelete {
                url: self.config.discord_webhook.clone(),
                message_id: existing.discord_id,
                thread_id: self.config.discord_thread_id,
                response: send,
            })
            .await?;
        recv.await?;
        Ok(())
    }

    async fn handle_discord_message_create(&mut self, message: DcMessage) -> Result<()> {
        let ly = self.globals.lamprey_handle().await?;
        let existing = self.globals.get_message_dc(message.id).await?;
        if existing.is_some() {
            debug!("message already bridged");
            return Ok(());
        }

        let mut puppet = ly
            .puppet_ensure(
                message.author.display_name().to_owned(),
                message.author.id.to_string(),
                self.room_id(),
                message.author.bot,
            )
            .await?;
        debug!("created puppet");
        let user_id = puppet.id;
        let p = self
            .globals
            .get_puppet("discord", &message.author.id.to_string())
            .await?;
        if let Some(p) = p {
            if p.ext_avatar != message.author.avatar_url() {
                if let Some(url) = message.author.avatar_url() {
                    info!("set user pfp for {}", user_id);
                    let name = if url.ends_with(".gif") {
                        "avatar.gif"
                    } else {
                        "avatar.png"
                    };
                    let bytes = reqwest::get(url).await?.error_for_status()?.bytes().await?;
                    let media = ly
                        .media_upload(name.to_owned(), bytes.to_vec(), user_id)
                        .await?;
                    ly.user_update(
                        user_id,
                        &types::UserPatch {
                            name: None,
                            description: None,
                            avatar: Some(Some(media.id)),
                        },
                    )
                    .await?;
                    puppet.avatar = Some(media.id);
                } else {
                    info!("remove user pfp for {}", user_id);
                    ly.user_update(
                        user_id,
                        &types::UserPatch {
                            name: None,
                            description: None,
                            avatar: Some(None),
                        },
                    )
                    .await?;
                    puppet.avatar = None;
                }
            }
        } else if message.author.avatar_url().is_some() {
            if let Some(url) = message.author.avatar_url() {
                info!("set user pfp for {}", user_id);
                let name = if url.ends_with(".gif") {
                    "avatar.gif"
                } else {
                    "avatar.png"
                };
                let bytes = reqwest::get(url).await?.error_for_status()?.bytes().await?;
                info!("set user pfp download");
                let media = ly
                    .media_upload(name.to_owned(), bytes.to_vec(), user_id)
                    .await?;
                info!("set user pfp upload");
                ly.user_update(
                    user_id,
                    &types::UserPatch {
                        name: None,
                        description: None,
                        avatar: Some(Some(media.id)),
                    },
                )
                .await?;
                info!("set user pfp patch");
                puppet.avatar = Some(media.id);
            } else {
                info!("remove user pfp for {}", user_id);
                ly.user_update(
                    user_id,
                    &types::UserPatch {
                        name: None,
                        description: None,
                        avatar: Some(None),
                    },
                )
                .await?;
                puppet.avatar = None;
            }
        }
        self.globals
            .insert_puppet(Puppet {
                id: *user_id,
                ext_platform: "discord".to_owned(),
                ext_id: message.author.id.to_string(),
                ext_avatar: message.author.avatar_url(),
                name: puppet.name,
                avatar: puppet.avatar.map(|a| a.to_string()),
                bot: Some(puppet.bot.is_some()),
            })
            .await?;
        debug!("inserted puppet");

        let mut req = types::MessageCreate {
            content: None,
            attachments: vec![],
            metadata: None,
            reply_id: None,
            override_name: message
                .member
                .and_then(|m| m.nick)
                .or(message.author.global_name)
                .or(Some(message.author.name.clone())),
            nonce: None,
            embeds: vec![],
            created_at: Some(
                OffsetDateTime::from_unix_timestamp(message.timestamp.unix_timestamp())
                    .unwrap()
                    .into(),
            ),
        };
        for a in &message.attachments {
            let bytes = a.download().await?;
            debug!("downloaded attachment");
            let media = ly
                .media_upload(a.filename.to_owned(), bytes.into(), user_id)
                .await?;
            debug!("reuploaded attachment");
            self.globals
                .insert_attachment(AttachmentMetadata {
                    chat_id: media.id,
                    discord_id: a.id,
                })
                .await?;
            debug!("saved attachment metadata to db");
            req.attachments.push(MediaRef { id: media.id });
        }
        for emb in message.embeds.iter().cloned() {
            let author_avatar = if let Some(url) = emb
                .author
                .as_ref()
                .and_then(|a| a.proxy_icon_url.as_deref())
            {
                let filename = Url::from_str(url)?
                    .path_segments()
                    .unwrap()
                    .last()
                    .unwrap()
                    .to_owned();
                let bytes = reqwest::get(url).await?.bytes().await?;
                let media = ly
                    .media_upload(filename.to_owned(), bytes.into(), user_id)
                    .await?;
                Some(MediaRef { id: media.id })
            } else {
                None
            };
            let create = EmbedCreate {
                url: emb.url.and_then(|u| u.parse().ok()),
                title: emb.title,
                description: emb.description,
                color: emb.colour.map(|c| format!("#{}", c.hex())),
                media: if let Some(url) = emb.image.as_ref().and_then(|i| i.proxy_url.as_deref()) {
                    let filename = Url::from_str(url)?
                        .path_segments()
                        .unwrap()
                        .last()
                        .unwrap()
                        .to_owned();
                    let bytes = reqwest::get(url).await?.bytes().await?;
                    let media = ly
                        .media_upload(filename.to_owned(), bytes.into(), user_id)
                        .await?;
                    Some(MediaRef { id: media.id })
                } else {
                    None
                },
                thumbnail: if let Some(url) =
                    emb.thumbnail.as_ref().and_then(|t| t.proxy_url.as_deref())
                {
                    let filename = Url::from_str(url)?
                        .path_segments()
                        .unwrap()
                        .last()
                        .unwrap()
                        .to_owned();
                    let bytes = reqwest::get(url).await?.bytes().await?;
                    let media = ly
                        .media_upload(filename.to_owned(), bytes.into(), user_id)
                        .await?;
                    Some(MediaRef { id: media.id })
                } else {
                    None
                },
                author_name: emb.author.as_ref().map(|a| a.name.clone()),
                author_url: emb.author.and_then(|a| a.url).and_then(|u| u.parse().ok()),
                author_avatar,
            };
            req.embeds.push(create);
        }
        req.content = match message.kind {
            DcMessageType::Regular | DcMessageType::InlineReply => {
                if message.content.is_empty() {
                    if message.attachments.is_empty() && message.embeds.is_empty() {
                        Some("(sticker, poll, or other unsupported message type)".to_string())
                    } else {
                        None
                    }
                } else {
                    Some(message.content)
                }
            }
            other => Some(format!("(discord message: {:?})", other)),
        };
        match message.message_reference.map(|r| r.kind) {
            Some(MessageReferenceKind::Default) => {
                if let Some(reply) = message.referenced_message {
                    let row = self.globals.get_message_dc(reply.id).await?;
                    req.reply_id = row.map(|r| r.chat_id);
                }
            }
            Some(MessageReferenceKind::Forward) => {
                // TODO: support forwards once serenity supports them
            }
            Some(_) | None => {}
        };
        let thread_id = self.thread_id();
        let res = ly.message_create(thread_id, user_id, req).await?;
        debug!("sent message");
        self.globals
            .insert_message(MessageMetadata {
                chat_id: res.id,
                chat_thread_id: thread_id,
                discord_id: message.id,
                discord_channel_id: message.channel_id,
            })
            .await?;
        Ok(())
    }

    async fn handle_discord_message_update(&mut self, update: DcMessageUpdate) -> Result<()> {
        let ly = self.globals.lamprey_handle().await?;
        let existing = self.globals.get_message_dc(update.id).await?;
        let Some(existing) = existing else {
            debug!("message already bridged");
            return Ok(());
        };

        let message = ly
            .message_get(existing.chat_thread_id, existing.chat_id)
            .await?;
        let user_id = message.author_id;

        let mut req = types::MessagePatch {
            content: None,
            attachments: None,
            metadata: None,
            reply_id: None,
            override_name: None,
            embeds: None,
            edited_at: update.edited_timestamp.map(|t| {
                OffsetDateTime::from_unix_timestamp(t.unix_timestamp())
                    .unwrap()
                    .into()
            }),
        };
        req.attachments = if let Some(atts) = &update.attachments {
            let mut v = vec![];
            for att in atts {
                let existing = self.globals.get_attachment_dc(att.id).await?;
                if let Some(existing) = existing {
                    v.push(MediaRef {
                        id: existing.chat_id,
                    });
                    continue;
                }
                let bytes = att.download().await?;
                let media = ly
                    .media_upload(att.filename.to_owned(), bytes.into(), user_id)
                    .await?;
                self.globals
                    .insert_attachment(AttachmentMetadata {
                        chat_id: media.id,
                        discord_id: att.id,
                    })
                    .await?;
                v.push(MediaRef { id: media.id });
            }
            Some(v)
        } else {
            None
        };
        req.content = match update.kind {
            Some(k) => Some(match k {
                DcMessageType::Regular | DcMessageType::InlineReply
                    if update.content.as_ref().is_none_or(|c| c.is_empty())
                        && update.attachments.as_ref().is_none_or(|a| a.is_empty()) =>
                {
                    Some("(empty message, or sticker/embeds only)".to_string())
                }
                DcMessageType::Regular | DcMessageType::InlineReply => update.content.clone(),
                other => Some(format!("(discord message: {:?})", other)),
            }),
            None => None,
        };

        let thread_id = self.thread_id();
        ly.message_update(thread_id, existing.chat_id, user_id, req)
            .await?;
        Ok(())
    }

    async fn handle_discord_message_delete(&mut self, message_id: DcMessageId) -> Result<()> {
        let ly = self.globals.lamprey_handle().await?;
        let Some(existing) = self.globals.get_message_dc(message_id).await? else {
            debug!("message doesnt exist or is already deleted");
            return Ok(());
        };

        let message = ly
            .message_get(existing.chat_thread_id, existing.chat_id)
            .await?;
        let user_id = message.author_id;

        self.globals.delete_message_dc(message_id).await?;
        let thread_id = self.thread_id();
        ly.message_delete(thread_id, existing.chat_id, user_id)
            .await?;
        Ok(())
    }

    async fn handle_discord_reaction_add(&mut self, add_reaction: DcReaction) -> Result<()> {
        let ly = self.globals.lamprey_handle().await?;
        let Some(user_id) = add_reaction.user_id else {
            debug!("missing user_id");
            return Ok(());
        };

        let Some(message) = self.globals.get_message_dc(add_reaction.message_id).await? else {
            debug!("missing message");
            return Ok(());
        };

        let puppet = ly
            .puppet_ensure(
                add_reaction
                    .member
                    .as_ref()
                    .map(|m| {
                        m.nick
                            .as_deref()
                            .unwrap_or_else(|| &m.user.display_name())
                            .to_owned()
                    })
                    .unwrap_or_else(|| user_id.to_string()),
                user_id.to_string(),
                self.room_id(),
                add_reaction.member.as_ref().map_or(false, |m| m.user.bot),
            )
            .await?;

        ly.message_react(
            self.thread_id(),
            message.chat_id,
            puppet.id,
            add_reaction.emoji.to_string(),
        )
        .await?;
        Ok(())
    }

    async fn handle_discord_reaction_remove(&mut self, removed_reaction: DcReaction) -> Result<()> {
        let ly = self.globals.lamprey_handle().await?;
        let Some(user_id) = removed_reaction.user_id else {
            debug!("missing user_id");
            return Ok(());
        };

        let Some(message) = self
            .globals
            .get_message_dc(removed_reaction.message_id)
            .await?
        else {
            debug!("missing message");
            return Ok(());
        };

        let puppet = ly
            .puppet_ensure(
                removed_reaction
                    .member
                    .as_ref()
                    .map(|m| {
                        m.nick
                            .as_deref()
                            .unwrap_or_else(|| &m.user.display_name())
                            .to_owned()
                    })
                    .unwrap_or_else(|| user_id.to_string()),
                user_id.to_string(),
                self.room_id(),
                removed_reaction
                    .member
                    .as_ref()
                    .map_or(false, |m| m.user.bot),
            )
            .await?;

        ly.message_unreact(
            self.thread_id(),
            message.chat_id,
            puppet.id,
            removed_reaction.emoji.to_string(),
        )
        .await?;
        Ok(())
    }

    async fn handle_discord_typing(&mut self, user_id: serenity::model::id::UserId) -> Result<()> {
        let ly = self.globals.lamprey_handle().await?;
        let Some(puppet) = self
            .globals
            .get_puppet("discord", &user_id.to_string())
            .await?
        else {
            debug!("missing puppet");
            return Ok(());
        };

        ly.typing_start(self.thread_id(), puppet.id.into()).await?;
        Ok(())
    }
}
