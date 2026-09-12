use std::collections::HashMap;
use std::sync::Arc;

use opentelemetry::trace::FutureExt;
use serenity::all::{
    CreateAllowedMentions, CreateEmbed, EditAttachments, ExecuteWebhook, Mentionable,
};
use tokio::sync::broadcast;
use tracing::{Instrument, debug, error, warn};

use crate::bridge_old::{MessageData, Portal, PortalEvent, PortalHandle, PortalId};
use crate::prelude::*;
use crate::types::Platform;
use crate::util::mentions::MessageTransformer;

pub struct DiscordPortal {
    portal_id: PortalId,
    portal: Portal,
    handle: PortalHandle,
    http: Arc<serenity::all::Http>,
    cache: Arc<serenity::all::Cache>,
}

impl DiscordPortal {
    pub async fn spawn(
        portal_id: PortalId,
        portal: Portal,
        handle: PortalHandle,
        http: Arc<serenity::all::Http>,
        cache: Arc<serenity::all::Cache>,
    ) -> (PortalId, Result<()>) {
        let me = Self {
            portal_id,
            portal,
            handle,
            http,
            cache,
        };
        (portal_id, me.run().await)
    }

    async fn run(self) -> Result<()> {
        let mut events = self.handle.events.subscribe();
        let http_client = reqwest::Client::new();
        // TODO: set user-agent header for http_client?

        let discord_cfg = self.portal.discord.as_ref().unwrap();
        self.backfill()
            .instrument(tracing::debug_span!(
                "discord backfill",
                portal_id = %self.portal_id,
                channel_id = %discord_cfg.channel_id,
            ))
            .await?;

        loop {
            let event = match events.recv().await {
                Ok(e) => e,
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!(portal_id=%self.portal_id, n, "portal event receiver lagged, skipping");
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => break,
            };

            if let Err(err) = self.handle_event(&http_client, &event).await {
                error!("error while handling event {event:?}: {err}");
            }
        }

        Ok(())
    }

    async fn backfill(&self) -> Result<()> {
        // TODO: non-blocking backfill (see LampreyPortal comment)
        let discord_cfg = self.portal.discord.as_ref().unwrap();
        let mut last_id = discord_cfg.last_id;

        debug!(last_id=%last_id, "start backfill");

        loop {
            let messages = discord_cfg
                .channel_id
                .messages(
                    &self.http,
                    serenity::all::GetMessages::new().after(last_id).limit(100),
                )
                .await;
            let messages = match messages {
                Ok(m) => m,
                Err(e) => {
                    warn!(%last_id, "failed to fetch messages: {e:?}");
                    return Ok(());
                }
            };

            debug!(count=%messages.len(), "backfill messages");

            // break if messages is empty
            if messages.is_empty() {
                return Ok(());
            }

            let mut messages = messages;
            messages.reverse();
            let messages = messages;

            for message in messages {
                let message_id = message.id;
                let event = PortalEvent::MessageCreate(MessageData::Discord {
                    message: Box::new(message),
                });

                if self.handle.events.send(Arc::new(event)).is_err() {
                    error!(%message_id, "portal event queue is full, dropping message");
                }

                last_id = message_id;
            }
        }
    }

    async fn handle_event(&self, http_client: &reqwest::Client, event: &PortalEvent) -> Result<()> {
        match event {
            PortalEvent::Typing(_) => {
                // discord doesn't have any good way of bridging typing notifications
                // NOTE: maybe i could send typing notifs through the bridge bot if anyone is typing on lamprey?
            }
            PortalEvent::MessageCreate(data) => {
                // PERF: parse after checking MessageData::Discord
                let transformer = MessageTransformer::parse(data);

                let (msg, user, room_member, info) = match data {
                    MessageData::Lamprey {
                        message,
                        user,
                        room_member,
                        info,
                    } => (&**message, &**user, room_member.as_deref(), &**info),
                    MessageData::Discord { .. } => return Ok(()),
                };

                // check if message has already been bridged
                if self
                    .handle
                    .bridge
                    .db
                    .message_get_by_lamprey_id(self.portal_id, msg.id)
                    .await?
                    .is_some()
                {
                    return Ok(());
                }

                // PERF: don't fetch webhook every time, cache it (Webhook::from_url)
                let discord_cfg = self.portal.discord.as_ref().unwrap();
                let webhook_url = &discord_cfg.webhook_url;
                let webhook =
                    serenity::all::Webhook::from_url(&self.http, webhook_url.as_str()).await?;

                let msg_inner = match &msg.latest_version.message_type {
                    common::v1::types::MessageType::DefaultMarkdown(m)
                    | common::v1::types::MessageType::ThreadInitial(m) => m,
                    _ => {
                        debug!("unsupported lamprey message type");
                        // TODO: format and send anyways?
                        return Ok(());
                    }
                };

                let mut content = msg_inner.content.to_owned().unwrap_or_else(|| {
                    if msg_inner.attachments.is_empty()
                        && msg_inner.embeds.is_empty()
                        && msg_inner.components.is_empty()
                    {
                        "(no content?)".to_owned()
                    } else {
                        "".to_owned()
                    }
                });

                let username = room_member
                    .and_then(|rm| rm.override_name.clone())
                    .unwrap_or_else(|| user.name.clone());

                // TODO: proper url joining
                let avatar_url = user
                    .avatar
                    .as_ref()
                    .map(|media_id| format!("{}/thumb/{}", info.cdn_url, media_id));

                let mut embeds = vec![];
                if let Some(reply_id) = msg.reply_id() {
                    if let Some(bridge_msg) = self
                        .handle
                        .bridge
                        .db
                        .message_get_by_lamprey_id(self.portal_id, reply_id)
                        .await?
                    {
                        if let Some(discord_msg_id) = bridge_msg.discord_message_id {
                            let discord_msg = self
                                .http
                                .get_message(discord_cfg.channel_id, discord_msg_id)
                                .await?;
                            let reply_content = format_discord_reply_content(&discord_msg);

                            let author_display = if discord_msg.webhook_id.is_some() {
                                discord_msg.author.name.clone()
                            } else {
                                discord_msg.author.mention().to_string()
                            };

                            let description = format!(
                                "**[replying to](https://canary.discord.com/channels/{}/{}/{})** {} \n{}",
                                discord_cfg.guild_id,
                                discord_cfg.channel_id,
                                discord_msg_id,
                                author_display,
                                reply_content,
                            );
                            content = format!("{} {}", discord_msg.author.mention(), content);
                            embeds.push(CreateEmbed::new().description(description));
                            if let Some(att) = discord_msg.attachments.first() {
                                embeds.push(CreateEmbed::new().image(&att.url));
                            }
                        } else {
                            // TODO: handle unknown reply
                        }
                    }
                }

                // TODO: handle embeds (download, reupload)
                let mut files = vec![];
                for attachment in &msg_inner.attachments {
                    let common::v1::types::MessageAttachmentType::Media { media } = &attachment.ty;
                    // TODO: proper url joining info.cdn_url.join(...)
                    let url = format!("{}/media/{}", info.cdn_url, media.id);
                    if let Ok(response) = http_client.get(&url).send().await {
                        if let Ok(bytes) = response.bytes().await {
                            files.push(serenity::all::CreateAttachment::bytes(
                                bytes,
                                media.filename.clone(),
                            ));
                        }
                    }
                }

                let (parsed_content, allowed_mentions) = match transformer {
                    Some(t) => {
                        let mut user_mappings = HashMap::new();
                        for id in t.mentioned_users() {
                            if let Ok(Some(u)) = self
                                .handle
                                .bridge
                                .db
                                .puppet_get_by_lamprey_id(id.to_string())
                                .await
                            {
                                user_mappings.insert(id.to_string(), u.discord_id);
                            }
                        }

                        // TODO: handle role and channel mappings

                        let (parsed, mentions) =
                            t.to_discord(&user_mappings, &HashMap::new(), &HashMap::new());
                        (parsed, mentions)
                    }
                    None => (content, CreateAllowedMentions::new()),
                };

                let mut builder = ExecuteWebhook::new()
                    .content(parsed_content)
                    .embeds(embeds)
                    .username(username)
                    .add_files(files)
                    .allowed_mentions(allowed_mentions);

                if let Some(avatar_url) = avatar_url {
                    builder = builder.avatar_url(avatar_url);
                }

                // TODO: handle threads (builder.in_thread(thread_id))
                // TODO: handle components (builder.components(components))

                let sent_message = webhook.execute(&self.http, true, builder).await?;

                if let Some(msg) = sent_message {
                    if let MessageData::Lamprey { message, .. } = data {
                        let mut attachments = vec![];
                        for (i, attachment) in msg_inner.attachments.iter().enumerate() {
                            let common::v1::types::MessageAttachmentType::Media { media } =
                                &attachment.ty;
                            if let Some(discord_att) = msg.attachments.get(i) {
                                attachments.push((media.id, discord_att.id));
                            }
                        }

                        let updated_message = crate::bridge_old::Message {
                            id: crate::types::MessageId::new(),
                            portal_id: self.portal_id,
                            source_platform: Platform::Lamprey,
                            lamprey_message_id: Some(message.id),
                            discord_message_id: Some(msg.id),
                            attachments,
                        };

                        let _ = self
                            .handle
                            .bridge
                            .db
                            .message_create(self.portal_id, updated_message)
                            .await;
                    }
                }
            }
            PortalEvent::MessageUpdate(data) => {
                let (msg, _user, _room_member, info) = match data {
                    MessageData::Lamprey {
                        message,
                        user,
                        room_member,
                        info,
                    } => (&**message, &**user, room_member.as_deref(), &**info),
                    MessageData::Discord { .. } => return Ok(()),
                };

                // PERF: don't fetch webhook every time, cache it (Webhook::from_url)
                let discord_cfg = self.portal.discord.as_ref().unwrap();
                let webhook_url = &discord_cfg.webhook_url;
                let webhook =
                    serenity::all::Webhook::from_url(&self.http, webhook_url.as_str()).await?;

                let Some(portal_msg) = self
                    .handle
                    .bridge
                    .db
                    .message_get_by_lamprey_id(self.portal_id, msg.id)
                    .await?
                else {
                    return Ok(());
                };

                let Some(message_id) = portal_msg.discord_message_id else {
                    return Ok(());
                };

                // TODO: deduplicate code with MessageCreate handler
                let msg_inner = match &msg.latest_version.message_type {
                    common::v1::types::MessageType::DefaultMarkdown(m)
                    | common::v1::types::MessageType::ThreadInitial(m) => m,
                    _ => {
                        debug!("unsupported lamprey message type");
                        // TODO: format and send anyways?
                        return Ok(());
                    }
                };

                let content = msg_inner.content.to_owned().unwrap_or_else(|| {
                    if msg_inner.attachments.is_empty()
                        && msg_inner.embeds.is_empty()
                        && msg_inner.components.is_empty()
                    {
                        "(no content?)".to_owned()
                    } else {
                        "".to_owned()
                    }
                });

                let mut attachments = EditAttachments::new();
                for attachment in &msg_inner.attachments {
                    let common::v1::types::MessageAttachmentType::Media { media } = &attachment.ty;

                    // Check if we already have this attachment
                    if let Some(discord_id) = portal_msg
                        .attachments
                        .iter()
                        .find(|(l_id, _)| l_id == &media.id)
                        .map(|(_, d_id)| *d_id)
                    {
                        attachments = attachments.keep(discord_id);
                        continue;
                    }

                    // Otherwise, download and add
                    let url = format!("{}/media/{}", info.cdn_url, media.id);
                    if let Ok(response) = http_client.get(&url).send().await {
                        if let Ok(bytes) = response.bytes().await {
                            attachments = attachments.add(serenity::all::CreateAttachment::bytes(
                                bytes,
                                media.filename.clone(),
                            ));
                        }
                    }
                }

                let edited = webhook
                    .edit_message(
                        &self.http,
                        message_id,
                        serenity::all::EditWebhookMessage::new()
                            .content(content)
                            .attachments(attachments)
                            .allowed_mentions(CreateAllowedMentions::new()),
                    )
                    .await?;

                let mut new_attachments = vec![];
                for (i, attachment) in msg_inner.attachments.iter().enumerate() {
                    let common::v1::types::MessageAttachmentType::Media { media } = &attachment.ty;
                    if let Some(discord_att) = edited.attachments.get(i) {
                        new_attachments.push((media.id, discord_att.id));
                    }
                }

                let updated_message = crate::bridge_old::Message {
                    attachments: new_attachments,
                    ..portal_msg
                };

                // WARNING: if the bridge ever has more than two endpoints, i need to handle race conditions/conflicts/overwriting here
                let _ = self
                    .handle
                    .bridge
                    .db
                    .message_update(self.portal_id, updated_message)
                    .await;
            }
            PortalEvent::MessageDelete(message_id) => {
                if let Some(discord_cfg) = self.portal.discord.as_ref() {
                    if let Some(msg) = self.handle.bridge.db.message_get(*message_id).await? {
                        if let (Some(lamprey_msg_id), Some(discord_message_id)) =
                            (msg.lamprey_message_id, msg.discord_message_id)
                        {
                            let _ = self
                                .http
                                .delete_message(discord_cfg.channel_id, discord_message_id, None)
                                .await;
                            let _ = self
                                .handle
                                .bridge
                                .db
                                .message_delete_by_lamprey(self.portal_id, lamprey_msg_id)
                                .await;
                        }
                    }
                }
            }
            // TODO: how do i handle lamprey -> discord reactions?
            // PortalEvent::ReactionCreate(message_id, reaction_key, _user) => {
            //     if let Some(discord_cfg) = portal.discord.as_ref() {
            //         if let Ok(Some(msg)) = handle.bridge.db.message_get(*message_id).await {
            //             if let Some(discord_message_id) = msg.discord_message_id {
            //                 let key = match reaction_key {
            //                     bridge::ReactionKey::Lamprey(key) => match &key {
            //                         lamprey::ReactionKey::Text { content }
            //                             if key.is_unicode_emoji() =>
            //                         {
            //                             ReactionType::Unicode(content.to_owned())
            //                         }
            //                         // TODO: support custom reactions
            //                         // lamprey::ReactionKey::Custom(emoji) => todo!(),
            //                         _ => return Ok(()),
            //                     },
            //                     bridge::ReactionKey::Discord(_key) => return Ok(()),
            //                 };
            //
            //                 let _ = http
            //                     .create_reaction(discord_cfg.channel_id, discord_message_id, &key)
            //                     .await;
            //             }
            //         }
            //     }
            // }
            // PortalEvent::ReactionDelete(message_id, reaction_key, user) => {}
            // PortalEvent::ReactionDeleteKey(message_id, reaction_key) => {}
            // PortalEvent::ReactionDeleteAll(message_id, _) => {}
            _ => {}
        }
        Ok(())
    }
}

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
