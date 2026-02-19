use anyhow::Result;
use common::v1::types::{self};
use common::v2::types::message::Message;
use serenity::all::{
    CreateAllowedMentions, CreateAttachment, CreateEmbed, EditAttachments, EditWebhookMessage,
    ExecuteWebhook, Mentionable,
};
use tracing::debug;

use crate::db::{AttachmentMetadata, Data, MessageMetadata};
use crate::discord::DiscordMessage;
use crate::portal::Portal;

impl Portal {
    pub(super) async fn handle_lamprey_message_create(&mut self, message: Message) -> Result<()> {
        let ly = self.globals.lamprey_handle().await?;
        let user = ly.user_fetch(message.author_id).await?;
        if user.puppet.is_some() {
            debug!("not bridging message from puppet");
            return Ok(());
        }

        let existing = self.globals.get_message(message.id).await?;
        let msg_inner = match message.latest_version.message_type {
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
                chat_thread_id: message.channel_id,
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

    pub(super) async fn handle_lamprey_message_delete(
        &mut self,
        message_id: common::v1::types::MessageId,
    ) -> Result<()> {
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
}

use tokio::sync::oneshot;
