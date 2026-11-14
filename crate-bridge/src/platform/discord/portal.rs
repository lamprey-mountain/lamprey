use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use opentelemetry::trace::FutureExt;
use tokio::sync::broadcast;
use tracing::{Instrument, debug, error, warn};
use twilight_http::Client as HttpClient;
use twilight_model::channel::message::AllowedMentions;
use twilight_model::channel::message::embed::{Embed, EmbedImage};
use twilight_model::channel::{ChannelType, Message};
use twilight_model::http::attachment::Attachment as HttpAttachment;
use twilight_model::id::Id;

use crate::bridge_old::{
    MessageData, Portal, PortalEvent, PortalEventEnvelope, PortalHandle, PortalId, ThreadId,
};
use crate::platform::discord::{ChannelId, WebhookId};
use crate::prelude::*;
use crate::types::Platform;
use crate::util::mentions::MessageTransformer;

pub struct DiscordPortal {
    portal_id: PortalId,
    portal: Portal,
    handle: PortalHandle,
    http: Arc<HttpClient>,
    /// last lamprey message id that was bridged, used to keep the persisted
    /// backfill cursor monotonic
    lamprey_cursor: Mutex<lamprey::MessageId>,
}

impl DiscordPortal {
    pub async fn spawn(
        portal_id: PortalId,
        portal: Portal,
        handle: PortalHandle,
        http: Arc<HttpClient>,
    ) -> (PortalId, Result<()>) {
        let lamprey_cursor = Mutex::new(
            portal
                .lamprey
                .as_ref()
                .expect("discord portal has a lamprey side")
                .last_id,
        );

        let me = Self {
            portal_id,
            portal,
            handle,
            http,
            lamprey_cursor,
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
            let envelope = match events.recv().await {
                Ok(e) => e,
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!(portal_id=%self.portal_id, n, "portal event receiver lagged, skipping");
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => break,
            };

            if let Err(err) = self
                .handle_event(&http_client, &envelope.event, envelope.thread)
                .await
            {
                error!("error while handling event {:?}: {err}", envelope.event);
            }
        }

        Ok(())
    }

    async fn backfill(&self) -> Result<()> {
        // TODO: non-blocking backfill (see LampreyPortal comment)
        let discord_cfg = self.portal.discord.as_ref().unwrap();
        // prefer the persisted cursor so we catch up after a reconnect
        let mut last_id = self
            .handle
            .bridge
            .db
            .portal_get_by_discord_channel(discord_cfg.channel_id.to_string())
            .await?
            .and_then(|portal| portal.discord)
            .map(|discord| discord.last_id)
            .unwrap_or(discord_cfg.last_id);

        debug!(last_id=%last_id, "start backfill");

        loop {
            let messages = self
                .http
                .channel_messages(discord_cfg.channel_id)
                .after(last_id)
                .limit(100)
                .await;
            let messages = match messages {
                Ok(r) => match r.model().await {
                    Ok(m) => m,
                    Err(e) => {
                        warn!(%last_id, "failed to fetch messages: {e:?}");
                        return Ok(());
                    }
                },
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

                let envelope = PortalEventEnvelope {
                    event,
                    thread: None,
                };
                if self.handle.events.send(Arc::new(envelope)).is_err() {
                    error!(%message_id, "portal event queue is full, dropping message");
                }

                last_id = message_id;
            }
        }
    }

    /// persist the backfill cursor so reconnects don't replay old messages
    async fn update_lamprey_cursor(&self, last_id: lamprey::MessageId) {
        {
            let mut cursor = self.lamprey_cursor.lock().unwrap();
            if last_id <= *cursor {
                return;
            }
            *cursor = last_id;
        }

        if let Err(err) = self
            .handle
            .bridge
            .db
            .portal_update_lamprey_last_id(self.portal_id, last_id)
            .await
        {
            warn!(portal_id = %self.portal_id, "failed to update lamprey cursor: {err}");
        }
    }

    // Message-based thread channel ids equal their starter message id; private
    // threads (created via `thread_create`) have their own portal instead.
    async fn resolve_thread(&self, thread: Option<ThreadId>) -> Result<Option<ChannelId>> {
        let Some(ThreadId::Lamprey(channel_id)) = thread else {
            return Ok(None);
        };

        let message_id = lamprey::MessageId::from(channel_id.into_inner());
        let discord_thread_id = self
            .handle
            .bridge
            .db
            .message_get_by_lamprey_id(self.portal_id, message_id)
            .await?
            .and_then(|msg| msg.discord_message_id)
            .map(|id| Id::new(id.get()));

        if discord_thread_id.is_none() {
            debug!(%channel_id, "no bridged message for thread, falling back to parent channel");
        }

        Ok(discord_thread_id)
    }

    async fn handle_event(
        &self,
        http_client: &reqwest::Client,
        event: &PortalEvent,
        thread: Option<ThreadId>,
    ) -> Result<()> {
        match event {
            PortalEvent::Backfill => {
                self.backfill().await?;
            }
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
                    self.update_lamprey_cursor(msg.id).await;
                    return Ok(());
                }

                let discord_cfg = self.portal.discord.as_ref().unwrap();
                let (webhook_id, webhook_token) = parse_webhook_url(&discord_cfg.webhook_url)?;
                let thread_id = self.resolve_thread(thread).await?;
                let channel_id = thread_id.unwrap_or(discord_cfg.channel_id);

                let msg_inner = match &msg.latest_version.message_type {
                    common::v1::types::MessageType::DefaultMarkdown(m)
                    | common::v1::types::MessageType::ThreadInitial(m) => m,
                    _ => {
                        debug!("unsupported lamprey message type");
                        // TODO: format and send anyways?
                        self.update_lamprey_cursor(msg.id).await;
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
                                .message(channel_id, discord_msg_id)
                                .await?
                                .model()
                                .await?;
                            let reply_content = format_discord_reply_content(&discord_msg);

                            let author_display = if discord_msg.webhook_id.is_some() {
                                discord_msg.author.name.clone()
                            } else {
                                format!("<@{}>", discord_msg.author.id)
                            };

                            let description = format!(
                                "**[replying to](https://canary.discord.com/channels/{}/{}/{})** {} \n{}",
                                discord_cfg.guild_id,
                                channel_id,
                                discord_msg_id,
                                author_display,
                                reply_content,
                            );
                            content = format!("<@{}> {}", discord_msg.author.id, content);
                            embeds.push(embed_with_description(description));
                            if let Some(att) = discord_msg.attachments.first() {
                                embeds.push(embed_with_image(att.url.clone()));
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
                            let id = files.len() as u64;
                            files.push(http_attachment(media.filename.clone(), bytes.to_vec(), id));
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
                    None => (content, AllowedMentions::default()),
                };

                let mut builder = self
                    .http
                    .execute_webhook(webhook_id, &webhook_token)
                    .content(&parsed_content)
                    .embeds(&embeds)
                    .username(&username)
                    .attachments(&files)
                    .allowed_mentions(Some(&allowed_mentions));

                if let Some(ref avatar_url) = avatar_url {
                    builder = builder.avatar_url(avatar_url);
                }

                if let Some(thread_id) = thread_id {
                    builder = builder.thread_id(thread_id);
                } else if discord_cfg.parent_id.is_some() {
                    builder = builder.thread_id(discord_cfg.channel_id);
                }

                // TODO: handle components (builder.components(components))

                let sent_message = builder.wait().await?.model().await?;

                if let MessageData::Lamprey { message, .. } = data {
                    let mut attachments = vec![];
                    for (i, attachment) in msg_inner.attachments.iter().enumerate() {
                        let common::v1::types::MessageAttachmentType::Media { media } =
                            &attachment.ty;
                        if let Some(discord_att) = sent_message.attachments.get(i) {
                            attachments.push((media.id, discord_att.id));
                        }
                    }

                    let updated_message = crate::bridge_old::Message {
                        id: crate::types::MessageId::new(),
                        portal_id: self.portal_id,
                        source_platform: Platform::Lamprey,
                        lamprey_message_id: Some(message.id),
                        discord_message_id: Some(sent_message.id),
                        attachments,
                    };

                    if let Err(err) = self
                        .handle
                        .bridge
                        .db
                        .message_create(self.portal_id, updated_message)
                        .await
                    {
                        error!(portal_id = %self.portal_id, "failed to store bridged message: {err}");
                    }
                }

                self.update_lamprey_cursor(msg.id).await;
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

                let discord_cfg = self.portal.discord.as_ref().unwrap();
                let (webhook_id, webhook_token) = parse_webhook_url(&discord_cfg.webhook_url)?;
                let thread_id = self.resolve_thread(thread).await?;

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

                let mut keep = vec![];
                let mut files = vec![];
                for attachment in &msg_inner.attachments {
                    let common::v1::types::MessageAttachmentType::Media { media } = &attachment.ty;

                    // Check if we already have this attachment
                    if let Some(discord_id) = portal_msg
                        .attachments
                        .iter()
                        .find(|(l_id, _)| l_id == &media.id)
                        .map(|(_, d_id)| *d_id)
                    {
                        keep.push(discord_id);
                        continue;
                    }

                    // Otherwise, download and add
                    let url = format!("{}/media/{}", info.cdn_url, media.id);
                    if let Ok(response) = http_client.get(&url).send().await {
                        if let Ok(bytes) = response.bytes().await {
                            let id = files.len() as u64;
                            files.push(http_attachment(media.filename.clone(), bytes.to_vec(), id));
                        }
                    }
                }

                let allowed_mentions = AllowedMentions::default();
                let mut edit_builder = self
                    .http
                    .update_webhook_message(webhook_id, &webhook_token, message_id)
                    .content(Some(&content))
                    .attachments(&files)
                    .keep_attachment_ids(&keep)
                    .allowed_mentions(Some(&allowed_mentions));
                if let Some(thread_id) = thread_id {
                    edit_builder = edit_builder.thread_id(thread_id);
                } else if discord_cfg.parent_id.is_some() {
                    edit_builder = edit_builder.thread_id(discord_cfg.channel_id);
                }
                let edited = edit_builder.await?.model().await?;

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
                            let channel_id = self
                                .resolve_thread(thread)
                                .await?
                                .unwrap_or(discord_cfg.channel_id);
                            let _ = self
                                .http
                                .delete_message(channel_id, discord_message_id)
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
            PortalEvent::ThreadCreate(chan) => {
                // make sure that only one thread can be created at a time
                let _guard = self.handle.bridge.channel_lock.lock().await;

                let (name, lamprey_chan, source_message_id, kind) = match chan {
                    bridge_old::ChannelData::Discord { .. } => return Ok(()),
                    bridge_old::ChannelData::Lamprey { channel } => {
                        if self
                            .handle
                            .bridge
                            .db
                            .portal_get_by_lamprey_channel(channel.id.to_string())
                            .await?
                            .is_some()
                        {
                            return Ok(());
                        }

                        // threads created from a message use the source message id as
                        // the thread channel id, so look up the bridged message to
                        // attach the discord thread to it
                        let source_message_id = if channel.ty == lamprey::ChannelType::ThreadPublic
                        {
                            self.handle
                                .bridge
                                .db
                                .message_get_by_lamprey_id(
                                    self.portal_id,
                                    lamprey::MessageId::from(channel.id.into_inner()),
                                )
                                .await?
                                .and_then(|msg| msg.discord_message_id)
                        } else {
                            None
                        };

                        let name = channel
                            .name
                            .clone()
                            .or_else(|| channel.auto_name.clone())
                            .unwrap_or_else(|| "thread".to_string());

                        let kind = match channel.ty {
                            lamprey::ChannelType::ThreadPublic => ChannelType::PublicThread,
                            lamprey::ChannelType::ThreadPrivate => ChannelType::PrivateThread,
                            // TODO: create AnnouncementThread in News (announcement) channels
                            _ => return Ok(()),
                        };
                        (name, channel, source_message_id, kind)
                    }
                };

                let discord_cfg = self.portal.discord.as_ref().unwrap();

                let thread = match source_message_id {
                    Some(message_id) => {
                        self.http
                            .create_thread_from_message(discord_cfg.channel_id, message_id, &name)
                            .await?
                            .model()
                            .await?
                    }
                    None => {
                        self.http
                            .create_thread(discord_cfg.channel_id, &name, kind)
                            .await?
                            .model()
                            .await?
                    }
                };

                let portal_id = bridge_old::PortalId::new();
                let portal = bridge_old::Portal {
                    id: portal_id,
                    realm_id: self.portal.realm_id,
                    lamprey: Some(bridge_old::PortalLamprey {
                        channel_id: lamprey_chan.id,
                        room_id: self.portal.lamprey.as_ref().unwrap().room_id,
                        last_id: lamprey_chan.last_message_id.unwrap_or_default(),
                    }),
                    discord: Some(bridge_old::PortalDiscord {
                        guild_id: discord_cfg.guild_id,
                        parent_id: Some(discord_cfg.channel_id),
                        channel_id: thread.id,
                        webhook_url: discord_cfg.webhook_url.clone(),
                        webhook_id: discord_cfg.webhook_id,
                        last_id: thread
                            .last_message_id
                            .map(|id| id.cast())
                            .unwrap_or(Id::new(0)),
                    }),
                };

                // the thread is already created on discord, so a failure here
                // leaves it without a portal (and it will be created again on the
                // next restart)
                match self.handle.bridge.db.portal_create(portal.clone()).await {
                    Ok(()) => {
                        let handle = self.handle.bridge.create_portal_handle(portal.id);
                        let _ = self.handle.bridge.events.send(Arc::new(
                            bridge_old::BridgeEvent::PortalCreated(portal, handle),
                        ));
                    }
                    Err(err) => error!(
                        portal_id = %portal.id,
                        "failed to store portal for newly created thread: {err}"
                    ),
                }
            }
            _ => {}
        }
        Ok(())
    }
}

fn parse_webhook_url(url: &url::Url) -> Result<(WebhookId, String)> {
    // skip empty segments so trailing slashes and doubled slashes still parse
    let mut segments = url
        .path_segments()
        .into_iter()
        .flatten()
        .filter(|segment| !segment.is_empty())
        .rev();
    let token = segments
        .next()
        .ok_or_else(|| anyhow::anyhow!("invalid webhook url"))?
        .to_owned();
    let id: WebhookId = segments
        .next()
        .ok_or_else(|| anyhow::anyhow!("invalid webhook url"))?
        .parse()?;
    Ok((id, token))
}

fn http_attachment(filename: String, bytes: Vec<u8>, id: u64) -> HttpAttachment {
    HttpAttachment {
        title: None,
        description: None,
        file: bytes,
        filename,
        id,
        duration_secs: None,
        waveform: None,
        is_spoiler: None,
    }
}

fn embed_with_description(description: String) -> Embed {
    Embed {
        author: None,
        color: None,
        description: Some(description),
        fields: vec![],
        flags: None,
        footer: None,
        image: None,
        kind: "rich".to_owned(),
        provider: None,
        thumbnail: None,
        timestamp: None,
        title: None,
        url: None,
        video: None,
    }
}

fn embed_with_image(url: String) -> Embed {
    Embed {
        author: None,
        color: None,
        description: None,
        fields: vec![],
        flags: None,
        footer: None,
        image: Some(EmbedImage {
            height: None,
            proxy_url: None,
            url,
            width: None,
            content_type: None,
            placeholder: None,
            placeholder_version: None,
            flags: None,
        }),
        kind: "rich".to_owned(),
        provider: None,
        thumbnail: None,
        timestamp: None,
        title: None,
        url: None,
        video: None,
    }
}

fn format_discord_reply_content(discord_msg: &Message) -> String {
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
