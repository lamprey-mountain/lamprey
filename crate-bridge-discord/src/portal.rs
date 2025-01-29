use std::sync::Arc;

use anyhow::Result;
use serenity::all::CreateAllowedMentions;
use serenity::all::CreateAttachment;
use serenity::all::CreateEmbed;
use serenity::all::EditAttachments;
use serenity::all::EditWebhookMessage;
use serenity::all::{
    ChannelId as DcChannelId, Message as DcMessage, MessageId as DcMessageId,
    MessageType as DcMessageType, MessageUpdateEvent as DcMessageUpdate,
};
use serenity::all::{ExecuteWebhook, MessageReferenceKind};
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tracing::error;
use types::Message;
use types::MessageId;
use types::ThreadId;

use crate::common::ConfigPortal;
use crate::common::Globals;
use crate::data::AttachmentMetadata;
use crate::data::Data;
use crate::data::MessageMetadata;
use crate::{chat::UnnamedMessage, discord::DiscordMessage};

/// a bidirectional portal to a sinister realm
pub struct Portal {
    globals: Arc<Globals>,
    recv: mpsc::UnboundedReceiver<PortalMessage>,
    config: ConfigPortal,
}

/// portal actor message
pub enum PortalMessage {
    UnnamedMessageUpsert { message: Message },
    UnnamedMessageDelete { message_id: MessageId },
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

    async fn activate(mut self) {
        while let Some(msg) = self.recv.recv().await {
            match self.handle(msg).await {
                Ok(_) => {},
                Err(err) => error!("{err}"),
            };
        }
    }

    async fn handle(&mut self, msg: PortalMessage) -> Result<()> {
        match msg {
            PortalMessage::UnnamedMessageUpsert { message } => {
                let existing = self.globals.get_message(message.id).await?;
                let reply_ids = if let Some(reply_id) = message.reply_id {
                    self.globals
                        .get_message(reply_id)
                        .await?
                        .map(|i| (i.discord_id, i.chat_id))
                } else {
                    None
                };
                let mut embeds = vec![];
                let thread_id = self.thread_id();
                if let Some(reply_ids) = reply_ids {
                    let (discord_id, chat_id) = reply_ids;
                    let (send, recv) = oneshot::channel();
                    self.globals
                        .ch_chan
                        .send(UnnamedMessage::MessageGet {
                            thread_id,
                            message_id: chat_id,
                            response: send,
                        })
                        .await?;
                    let reply = recv.await?;
                    let description = format!(
                        "**[replying to {}](https://canary.discord.com/channels/{}/{}/{})**\n{}",
                        reply.override_name.unwrap_or(reply.author.name),
                        self.config.discord_guild_id,
                        self.channel_id(),
                        discord_id,
                        reply.content.as_deref().unwrap_or("(no content?)")
                    );
                    embeds.push(CreateEmbed::new().description(description));
                }
                let (send, recv) = tokio::sync::oneshot::channel();
                if let Some(edit) = existing {
                    let mut files = EditAttachments::new();
                    for media in &message.attachments {
                        let existing = self.globals.get_attachment(media.id.to_owned()).await?;
                        if let Some(existing) = existing {
                            files = files.keep(existing.discord_id);
                        } else {
                            let bytes = reqwest::get(media.url.to_owned())
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
                        .content(message.content.as_deref().unwrap_or("(no content?)"))
                        .allowed_mentions(
                            CreateAllowedMentions::new()
                                .everyone(false)
                                .all_roles(false)
                                .all_users(false),
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
                    for media in &message.attachments {
                        let bytes = reqwest::get(media.url.to_owned())
                            .await?
                            .error_for_status()?
                            .bytes()
                            .await?;
                        files.push(CreateAttachment::bytes(bytes, media.filename.to_owned()));
                    }
                    let mut payload = ExecuteWebhook::new()
                        .username(message.override_name.unwrap_or(message.author.name))
                        .content(message.content.as_deref().unwrap_or("(no content?)"))
                        .allowed_mentions(
                            CreateAllowedMentions::new()
                                .everyone(false)
                                .all_roles(false)
                                .all_users(false),
                        )
                        .add_files(files)
                        .embeds(embeds);
                    if let Some(dc_tid) = self.config.discord_thread_id {
                        payload = payload.in_thread(dc_tid);
                    }
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
                for (att, media) in res.attachments.iter().zip(message.attachments) {
                    self.globals
                        .insert_attachment(AttachmentMetadata {
                            chat_id: media.id,
                            discord_id: att.id,
                        })
                        .await?;
                }
            }
            PortalMessage::UnnamedMessageDelete { message_id } => {
                let Some(existing) = self.globals.get_message(message_id).await? else {
                    return Ok(());
                };
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
                let mut req = types::MessageCreateRequest {
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
                };
                for a in &message.attachments {
                    let bytes = a.download().await?;
                    let (send, recv) = tokio::sync::oneshot::channel();
                    self.globals
                        .ch_chan
                        .send(UnnamedMessage::MediaUpload {
                            filename: a.filename.to_owned(),
                            bytes,
                            response: send,
                        })
                        .await?;
                    let media = recv.await?;
                    self.globals
                        .insert_attachment(AttachmentMetadata {
                            chat_id: media.id,
                            discord_id: a.id,
                        })
                        .await?;
                    req.attachments.push(types::MediaRef { id: media.id });
                }
                req.content = Some(match message.kind {
                    DcMessageType::Regular | DcMessageType::InlineReply
                        if message.content.is_empty() && message.attachments.is_empty() =>
                    {
                        "(empty message, or sticker/embeds only)".to_string()
                    }
                    DcMessageType::Regular | DcMessageType::InlineReply => message.content,
                    other => format!("(discord message: {:?})", other),
                });
                match message.message_reference.map(|r| r.kind) {
                    Some(MessageReferenceKind::Default) => {
                        let reply = message
                            .referenced_message
                            .expect("replies should have a referenced message");
                        let row = self.globals.get_message_dc(reply.id).await?;
                        req.reply_id = row.map(|r| r.chat_id);
                    }
                    Some(MessageReferenceKind::Forward) => {
                        // TODO: support forwards once serenity supports them
                    }
                    Some(_) | None => {}
                };
                let (send, recv) = oneshot::channel();
                let thread_id = self.thread_id();
                self.globals
                    .ch_chan
                    .send(UnnamedMessage::MessageCreate {
                        thread_id,
                        req,
                        response: send,
                    })
                    .await?;
                let res = recv.await?;
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
                let mut req = types::MessagePatch {
                    content: None,
                    attachments: None,
                    metadata: None,
                    reply_id: None,
                    override_name: None,
                };
                if let Some(name) = update
                    .member
                    .flatten()
                    .and_then(|m| m.nick)
                    .or_else(|| {
                        update
                            .author
                            .as_ref()
                            .and_then(|u| u.global_name.to_owned())
                    })
                    .or_else(|| update.author.as_ref().map(|u| u.name.to_owned()))
                {
                    req.override_name = Some(Some(name));
                }
                req.attachments = if let Some(atts) = &update.attachments {
                    let mut v = vec![];
                    for att in atts {
                        let existing = self.globals.get_attachment_dc(att.id).await?;
                        if let Some(existing) = existing {
                            v.push(types::MediaRef {
                                id: existing.chat_id,
                            });
                            continue;
                        }
                        let bytes = att.download().await?;
                        let (send, recv) = tokio::sync::oneshot::channel();
                        self.globals
                            .ch_chan
                            .send(UnnamedMessage::MediaUpload {
                                filename: att.filename.to_owned(),
                                bytes,
                                response: send,
                            })
                            .await?;
                        let media = recv.await?;
                        self.globals
                            .insert_attachment(AttachmentMetadata {
                                chat_id: media.id,
                                discord_id: att.id,
                            })
                            .await?;
                        v.push(types::MediaRef { id: media.id });
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
                let (send, recv) = oneshot::channel();
                let thread_id = self.thread_id();
                self.globals
                    .ch_chan
                    .send(UnnamedMessage::MessageUpdate {
                        thread_id,
                        message_id: existing.chat_id,
                        req,
                        response: send,
                    })
                    .await?;
                recv.await?;
            }
            PortalMessage::DiscordMessageDelete { message_id } => {
                let Some(existing) = self.globals.get_message_dc(message_id).await? else {
                    return Ok(());
                };
                let (send, recv) = oneshot::channel();
                let thread_id = self.thread_id();
                self.globals
                    .ch_chan
                    .send(UnnamedMessage::MessageDelete {
                        thread_id,
                        message_id: existing.chat_id,
                        response: send,
                    })
                    .await?;
                recv.await?;
            }
        }
        Ok(())
    }
}
