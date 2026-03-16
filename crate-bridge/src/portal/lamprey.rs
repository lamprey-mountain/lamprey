use anyhow::Result;
use common::v2::types::message::Message;
use futures::future::try_join_all;
use serenity::all::{
    CreateAllowedMentions, CreateAttachment, CreateEmbed, EditAttachments, EditWebhookMessage,
    ExecuteWebhook, Mentionable,
};
use tracing::debug;

use crate::db::{AttachmentMetadata, Data, MessageMetadata};
use crate::portal::Portal;

/// Format reply content from a Discord message for display in a reply embed
fn format_discord_reply_content(discord_msg: &serenity::all::Message) -> String {
    if !discord_msg.content.is_empty() {
        discord_msg.content.to_owned()
    } else if !discord_msg.attachments.is_empty() {
        let names: Vec<_> = discord_msg
            .attachments
            .iter()
            .map(|a| a.filename.to_owned())
            .collect();
        format!(
            "{} attachment(s): {}",
            discord_msg.attachments.len(),
            names.join(", ")
        )
    } else if !discord_msg.embeds.is_empty() {
        format!("{} embed(s)", discord_msg.embeds.len())
    } else {
        "(no content?)".to_owned()
    }
}

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
            common::v2::types::message::MessageType::DefaultMarkdown(m) => m,
            _ => {
                debug!("unsupported lamprey message type");
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
            // Get the reply message using Discord actor
            let discord_msg = crate::discord::discord_get_message(
                self.globals.clone(),
                self.channel_id(),
                discord_id,
            )
            .await?;
            let reply_content = format_discord_reply_content(&discord_msg);
            let description = format!(
                "**[replying to](https://canary.discord.com/channels/{}/{}/{})**\n{}",
                self.config.discord_guild_id,
                self.channel_id(),
                discord_id,
                reply_content,
            );
            content = format!("{} {}", discord_msg.author.mention(), content);
            embeds.push(CreateEmbed::new().description(description));

            if let Some(att) = discord_msg.attachments.first() {
                embeds.push(CreateEmbed::new().image(&att.url));
            }
        }
        if let Some(edit) = existing {
            let mut files = EditAttachments::new();
            for attachment in &msg_inner.attachments {
                let common::v2::types::message::MessageAttachmentType::Media { media } =
                    &attachment.ty;
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
                    let bytes = self
                        .globals
                        .reqwest_client
                        .get(&url)
                        .send()
                        .await?
                        .error_for_status()?
                        .bytes()
                        .await?;
                    files = files.add(CreateAttachment::bytes(bytes, media.filename.to_owned()));
                }
            }
            // Edit using Discord actor
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
            let discord_msg = crate::discord::discord_edit_message(
                self.globals.clone(),
                self.config.discord_webhook.clone(),
                edit.discord_id,
                payload,
            )
            .await?;
            self.globals
                .insert_message(MessageMetadata {
                    chat_id: message.id,
                    chat_thread_id: message.channel_id,
                    discord_id: discord_msg.id,
                    discord_channel_id: discord_msg.channel_id,
                })
                .await?;

            for (att, attachment) in discord_msg.attachments.iter().zip(msg_inner.attachments) {
                let common::v2::types::message::MessageAttachmentType::Media { media } =
                    attachment.ty;
                self.globals
                    .insert_attachment(AttachmentMetadata {
                        chat_id: media.id,
                        discord_id: att.id,
                    })
                    .await?;
            }
        } else {
            // Download attachments concurrently for better performance
            let download_futures = msg_inner.attachments.iter().map(|attachment| {
                let globals = &self.globals;
                async move {
                    let common::v2::types::message::MessageAttachmentType::Media { media } =
                        &attachment.ty;
                    let url = format!(
                        "{}/media/{}",
                        globals
                            .config
                            .lamprey_cdn_url
                            .as_deref()
                            .unwrap_or("https://chat-cdn.celery.eu.org"),
                        media.id
                    );
                    let bytes = globals
                        .reqwest_client
                        .get(&url)
                        .send()
                        .await?
                        .error_for_status()?
                        .bytes()
                        .await?;
                    Ok::<_, anyhow::Error>(CreateAttachment::bytes(
                        bytes,
                        media.filename.to_owned(),
                    ))
                }
            });

            let files = try_join_all(download_futures).await?;

            let user = ly.user_fetch(message.author_id).await?;
            let mut payload = ExecuteWebhook::new()
                .username(user.name)
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
            // Execute using Discord actor
            let discord_msg = crate::discord::discord_execute_webhook(
                self.globals.clone(),
                self.config.discord_webhook.clone(),
                payload,
            )
            .await?;
            self.globals
                .insert_message(MessageMetadata {
                    chat_id: message.id,
                    chat_thread_id: message.channel_id,
                    discord_id: discord_msg.id,
                    discord_channel_id: discord_msg.channel_id,
                })
                .await?;

            for (att, attachment) in discord_msg.attachments.iter().zip(msg_inner.attachments) {
                let common::v2::types::message::MessageAttachmentType::Media { media } =
                    attachment.ty;
                self.globals
                    .insert_attachment(AttachmentMetadata {
                        chat_id: media.id,
                        discord_id: att.id,
                    })
                    .await?;
            }
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
        // Delete using Discord actor
        crate::discord::discord_delete_message(
            self.globals.clone(),
            self.config.discord_webhook.clone(),
            self.config.discord_thread_id,
            existing.discord_id,
        )
        .await?;
        Ok(())
    }
}
