use std::str::FromStr;
use std::sync::Arc;

use crate::common::ConfigPortal;
use crate::common::Globals;
use crate::data::AttachmentMetadata;
use crate::data::Data;
use crate::data::MessageMetadata;
use crate::data::Puppet;
use crate::discord::DiscordMessage;
use anyhow::Result;
use common::v1::types::media::MediaRef;
use common::v1::types::EmbedCreate;
use common::v1::types::RoomId;
use common::v1::types::{self, MediaTrackInfo, Message, MessageId, ThreadId};
use reqwest::Url;
use serenity::all::CreateAllowedMentions;
use serenity::all::CreateAttachment;
use serenity::all::CreateEmbed;
use serenity::all::EditAttachments;
use serenity::all::EditWebhookMessage;
use serenity::all::Mentionable;
use serenity::all::{
    ChannelId as DcChannelId, Message as DcMessage, MessageId as DcMessageId,
    MessageType as DcMessageType, MessageUpdateEvent as DcMessageUpdate,
};
use serenity::all::{ExecuteWebhook, MessageReferenceKind};
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tracing::error;
use tracing::info;

pub struct Portal {
    globals: Arc<Globals>,
    recv: mpsc::UnboundedReceiver<PortalMessage>,
    config: ConfigPortal,
}

/// portal actor message
pub enum PortalMessage {
    LampoMessageCreate { message: Message },
    LampoMessageUpdate { message: Message },
    LampoMessageDelete { message_id: MessageId },
    DiscordMessageCreate { message: DcMessage },
    DiscordMessageUpdate { update: DcMessageUpdate },
    DiscordMessageDelete { message_id: DcMessageId },
}

impl Portal {
    pub fn summon(
        globals: Arc<Globals>,
        config: ConfigPortal,
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
        self.config.my_thread_id
    }

    pub fn room_id(&self) -> RoomId {
        self.config.my_room_id
    }

    async fn activate(mut self) {
        while let Some(msg) = self.recv.recv().await {
            match self.handle(msg).await {
                Ok(_) => {}
                Err(err) => error!("{err}"),
            };
        }
    }

    async fn handle(&mut self, msg: PortalMessage) -> Result<()> {
        let ly = self.globals.lampo_handle().await?;
        match msg {
            // TODO: split apart
            PortalMessage::LampoMessageCreate { message }
            | PortalMessage::LampoMessageUpdate { message } => {
                let user = ly.user_fetch(message.author_id).await?;
                if user.puppet.is_some() {
                    return Ok(());
                }

                let existing = self.globals.get_message(message.id).await?;
                let msg_inner = match message.message_type {
                    types::MessageType::DefaultMarkdown(m) => m,
                    _ => todo!(),
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
                    content = format!("{} {}", reply.author.mention(), content);
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
                            let bytes = reqwest::get(media.source.url.to_owned())
                                .await?
                                .error_for_status()?
                                .bytes()
                                .await?;
                            files = files
                                .add(CreateAttachment::bytes(bytes, media.filename.to_owned()));
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
                        let bytes = reqwest::get(media.source.url.to_owned())
                            .await?
                            .error_for_status()?
                            .bytes()
                            .await?;
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
                        let avatar = ly.media_info(media_id).await?;
                        let valid_track = avatar
                            .all_tracks()
                            .find(|a| matches!(a.info, MediaTrackInfo::Image(_)));
                        if let Some(valid_track) = valid_track {
                            payload = payload.avatar_url(valid_track.url.as_str());
                        }
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
            }
            PortalMessage::LampoMessageDelete { message_id } => {
                let Some(existing) = self.globals.get_message(message_id).await? else {
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
            }
            PortalMessage::DiscordMessageCreate { message } => {
                let existing = self.globals.get_message_dc(message.id).await?;
                if existing.is_some() {
                    return Ok(());
                }

                let mut puppet = ly
                    .puppet_ensure(
                        message.author.display_name().to_owned(),
                        message.author.id.to_string(),
                        self.room_id(),
                    )
                    .await?;
                let user_id = puppet.id;
                let p = self
                    .globals
                    .get_puppet("discord", &message.author.id.to_string())
                    .await?;
                if let Some(p) = dbg!(p) {
                    if p.ext_avatar != message.author.avatar_url() {
                        if let Some(url) = message.author.avatar_url() {
                            info!("set user pfp for {user_id}");
                            let name = if url.ends_with(".gif") {
                                "avatar.gif"
                            } else {
                                "avatar.png"
                            };
                            let bytes =
                                reqwest::get(url).await?.error_for_status()?.bytes().await?;
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
                            info!("remove user pfp for {user_id}");
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
                        info!("set user pfp for {user_id}");
                        let name = if url.ends_with(".gif") {
                            "avatar.gif"
                        } else {
                            "avatar.png"
                        };
                        let bytes = reqwest::get(url).await?.error_for_status()?.bytes().await?;
                        info!("set user pfp download");
                        let media = dbg!(
                            ly.media_upload(name.to_owned(), bytes.to_vec(), user_id)
                                .await
                        )?;
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
                        info!("remove user pfp for {user_id}");
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

                let mut req = types::MessageCreate {
                    content: None,
                    attachments: vec![],
                    metadata: None,
                    reply_id: None,
                    override_name: message
                        .member
                        .and_then(|m| m.nick)
                        .or(message.author.global_name)
                        .or(Some(message.author.name)),
                    nonce: None,
                    embeds: vec![],
                };
                for a in &message.attachments {
                    let bytes = a.download().await?;
                    let media = ly
                        .media_upload(a.filename.to_owned(), bytes, user_id)
                        .await?;
                    self.globals
                        .insert_attachment(AttachmentMetadata {
                            chat_id: media.id,
                            discord_id: a.id,
                        })
                        .await?;
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
                        media: if let Some(url) =
                            emb.image.as_ref().and_then(|i| i.proxy_url.as_deref())
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
                                Some(
                                    "(sticker, poll, or other unsupported message type)"
                                        .to_string(),
                                )
                            } else {
                                Some("".to_string())
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
                self.globals
                    .insert_message(MessageMetadata {
                        chat_id: res.id,
                        chat_thread_id: thread_id,
                        discord_id: message.id,
                        discord_channel_id: message.channel_id,
                    })
                    .await?;
            }
            PortalMessage::DiscordMessageUpdate { update } => {
                let existing = self.globals.get_message_dc(update.id).await?;
                let Some(existing) = existing else {
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
                            .media_upload(att.filename.to_owned(), bytes, user_id)
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
                        DcMessageType::Regular | DcMessageType::InlineReply => update.content,
                        other => Some(format!("(discord message: {:?})", other)),
                    }),
                    None => None,
                };

                let thread_id = self.thread_id();
                ly.message_update(thread_id, existing.chat_id, user_id, req)
                    .await?;
            }
            PortalMessage::DiscordMessageDelete { message_id } => {
                let Some(existing) = self.globals.get_message_dc(message_id).await? else {
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
            }
        }
        Ok(())
    }
}
