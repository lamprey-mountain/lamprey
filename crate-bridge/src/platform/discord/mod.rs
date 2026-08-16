use std::collections::HashMap;
use std::sync::Arc;

use serenity::all::{
    CreateChannel, CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage,
    CreateWebhook, EditAttachments, ExecuteWebhook, GatewayIntents, Mentionable,
};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::JoinSet;
use tracing::{debug, error, info, warn};

use crate::actor::bridge::BridgeCommand;
use crate::bridge_old::{
    MessageData, PlatformHandle, Portal, PortalDiscord, PortalHandle, PortalId, PortalLamprey,
    Realm, RealmEvent, RealmHandle, RealmId,
};
use crate::config::Config;
use crate::platform::discord::events::DiscordEvent;
use crate::prelude::*;
use crate::types::{ChannelData, Platform};
use crate::util::mentions::MessageTransformer;
use crate::{
    bridge_old::{BridgeEvent, BridgeHandle, PortalEvent},
    config::DiscordConfig,
};

mod events;
mod interactions;

// re export discord (serenity) types
pub use serenity::all::{
    Activity, ActivityType, Attachment, AttachmentId, Channel, ChannelId, ChannelType,
    CreateAllowedMentions, CreateEmbed, Embed, GuildChannel, GuildId, Message, MessageId,
    OnlineStatus, Presence, RoleId, User, UserId, WebhookId,
};

pub fn spawn(bridge: BridgeHandle, config_full: Config, config: DiscordConfig) -> PlatformHandle {
    let (tx, rx) = oneshot::channel();
    let task = tokio::spawn(Discord::connect(bridge, config_full, config, tx));
    PlatformHandle {
        name: "discord",
        ready: rx,
        task,
    }
}

struct Discord {
    bridge: BridgeHandle,
    rx: mpsc::Receiver<events::DiscordEvent>,
    portal_tasks: JoinSet<(PortalId, Result<()>)>,
    realm_tasks: JoinSet<(RealmId, Result<()>)>,
    portal_handles: HashMap<PortalId, PortalHandle>,
    portal_lookup: HashMap<ChannelId, PortalId>,
    portal_data: HashMap<PortalId, Portal>,
    realm_handles: HashMap<RealmId, RealmHandle>,
    realm_lookup: HashMap<ChannelId, RealmId>,
    realm_data: HashMap<RealmId, Realm>,
    webhook_lookup: HashMap<serenity::all::WebhookId, PortalId>,
    http: Arc<serenity::all::Http>,
    cache: Arc<serenity::all::Cache>,
}

impl Discord {
    async fn connect(
        bridge: BridgeHandle,
        config_full: Config,
        config: DiscordConfig,
        ready_tx: oneshot::Sender<()>,
    ) -> Result<()> {
        let (tx, rx) = mpsc::channel(1024);
        let handler = events::Handler {
            tx,
            config: config_full,
        };
        let client = serenity::Client::builder(
            &config.token.load().expect("failed to load token"),
            GatewayIntents::all(),
        )
        .event_handler(handler)
        .await
        .map_err(|e| anyhow::anyhow!("Error creating client: {:?}", e))?;

        let http = client.http.clone();
        let cache = client.cache.clone();

        let me = Self {
            bridge,
            rx,
            portal_tasks: JoinSet::new(),
            realm_tasks: JoinSet::new(),
            portal_handles: HashMap::new(),
            portal_lookup: HashMap::new(),
            portal_data: HashMap::new(),
            realm_handles: HashMap::new(),
            realm_lookup: HashMap::new(),
            realm_data: HashMap::new(),
            webhook_lookup: HashMap::new(),
            http,
            cache,
        };
        me.start(client, ready_tx).await?;

        Ok(())
    }

    fn spawn_portal_task(&mut self, portal_id: PortalId) {
        let portal = self.portal_data.get(&portal_id).unwrap().clone();
        let handle = self.portal_handles.get(&portal_id).unwrap().clone();
        self.portal_tasks.spawn(spawn_portal(
            portal_id,
            portal,
            handle,
            self.http.clone(),
            self.cache.clone(),
        ));
    }

    fn spawn_realm_task(&mut self, realm_id: RealmId) {
        let realm = self.realm_data.get(&realm_id).unwrap().clone();
        let handle = self.realm_handles.get(&realm_id).unwrap().clone();
        self.realm_tasks.spawn(spawn_realm(
            realm_id,
            realm,
            handle,
            self.http.clone(),
            self.cache.clone(),
        ));
    }

    async fn start(
        mut self,
        mut client: serenity::Client,
        ready_tx: oneshot::Sender<()>,
    ) -> Result<()> {
        tokio::spawn(async move {
            if let Err(why) = client.start().await {
                eprintln!("Client error: {:?}", why);
            }
        });

        let mut bridge_events = self.bridge.events.subscribe();
        ready_tx.send(()).unwrap();

        loop {
            tokio::select! {
                Ok(event) = bridge_events.recv() => {
                    self.handle_bridge_event(&event).await;
                }
                Some(event) = self.rx.recv() => {
                    self.handle_discord_event(event).await;
                }
                Some(result) = self.portal_tasks.join_next() => {
                    match result {
                        Ok((portal_id, Err(e))) => {
                            warn!(%portal_id, "discord portal task failed: {e:?}");
                            self.spawn_portal_task(portal_id);
                        }
                        Ok((portal_id, Ok(()))) => {
                            debug!(%portal_id, "discord portal task exited cleanly");
                            self.portal_handles.remove(&portal_id);
                            self.portal_data.remove(&portal_id);
                            self.portal_lookup.retain(|_, v| *v != portal_id);
                            self.webhook_lookup.retain(|_, v| *v != portal_id);
                        }
                        Err(e) => {
                            warn!("discord portal task join error: {e:?}");
                        }
                    }
                },
                Some(result) = self.realm_tasks.join_next() => {
                    match result {
                        Ok((realm_id, Err(e))) => {
                            warn!(%realm_id, "discord realm task failed: {e:?}");
                            self.spawn_realm_task(realm_id);
                        }
                        Ok((realm_id, Ok(()))) => {
                            debug!(%realm_id, "discord realm task exited cleanly");
                            self.realm_handles.remove(&realm_id);
                            self.realm_data.remove(&realm_id);
                            self.realm_lookup.retain(|_, v| *v != realm_id);
                        }
                        Err(e) => {
                            warn!("discord realm task join error: {e:?}");
                        }
                    }
                },
            }
        }
    }

    async fn handle_discord_event(&mut self, event: DiscordEvent) {
        match event {
            DiscordEvent::MessageCreate(message) => {
                if let Some(webhook_id) = message.webhook_id {
                    if self.webhook_lookup.contains_key(&webhook_id) {
                        return;
                    }
                }

                self.route_portal_event(
                    message.channel_id,
                    PortalEvent::MessageCreate(MessageData::Discord {
                        message: Box::new(message),
                    }),
                );
            }
            DiscordEvent::MessageUpdate(event, new) => {
                // TODO: handle message updates without new (fetch from discord's api?)
                // TODO: at least warn if new doesnt exist right now
                if let Some(new_message) = new {
                    if let Some(webhook_id) = new_message.webhook_id {
                        if self.webhook_lookup.contains_key(&webhook_id) {
                            return;
                        }
                    }

                    self.route_portal_event(
                        event.channel_id,
                        PortalEvent::MessageUpdate(MessageData::Discord {
                            message: Box::new(new_message),
                        }),
                    );
                }
            }
            DiscordEvent::TypingStart(event) => {
                let discord_id = event.user_id.get().to_string();
                if let Ok(Some(user)) = self.bridge.db.puppet_get_by_discord_id(discord_id).await {
                    self.route_portal_event(event.channel_id, PortalEvent::Typing(user));
                }
            }
            DiscordEvent::MessageDelete(channel_id, message_id) => {
                if let Some(portal_id) = self.portal_lookup.get(&channel_id) {
                    if let Ok(Some(msg)) = self
                        .bridge
                        .db
                        .message_get_by_discord_id(*portal_id, message_id)
                        .await
                    {
                        self.route_portal_event(channel_id, PortalEvent::MessageDelete(msg.id));
                    }
                }
            }

            DiscordEvent::PresenceUpdate(presence) => {
                let _ = self
                    .bridge
                    .events
                    .send(Arc::new(BridgeEvent::PresenceUpdate(presence)));
            }
            DiscordEvent::ChannelCreate(channel) => {
                let has_continuous = self.realm_data.values().any(|r| r.continuous);
                if !has_continuous {
                    return;
                }

                let is_supported = matches!(
                    channel.kind,
                    ChannelType::Text | ChannelType::News | ChannelType::Category
                );
                if !is_supported {
                    return;
                }

                let is_text = matches!(channel.kind, ChannelType::Text | ChannelType::News);

                let Some(realm_id) = self.realm_data.iter().find_map(|(id, realm)| {
                    if let Some(r_discord) = &realm.discord {
                        if r_discord.guild_id == channel.guild_id && realm.continuous {
                            return Some(*id);
                        }
                    }
                    None
                }) else {
                    return;
                };

                let handle = self.realm_handles.get(&realm_id).unwrap().clone();
                let http = self.http.clone();

                tokio::spawn(async move {
                    let channel_data = if is_text {
                        let webhook = match channel
                            .create_webhook(&http, CreateWebhook::new("bridge"))
                            .await
                        {
                            Ok(wh) => wh,
                            Err(e) => {
                                error!(?e, "failed to create webhook");
                                return;
                            }
                        };

                        let webhook_url: url::Url = webhook
                            .url()
                            .expect("webhook url")
                            .parse()
                            .expect("invalid webhook url");

                        ChannelData::Discord {
                            channel: Box::new(channel.clone()),
                            webhook: Some((webhook.id, webhook_url)),
                        }
                    } else {
                        ChannelData::Discord {
                            channel: Box::new(channel.clone()),
                            webhook: None,
                        }
                    };

                    let _ = handle
                        .events
                        .send(Arc::new(RealmEvent::ChannelCreate(channel_data)));
                });
            }
            DiscordEvent::ChannelDelete(channel) => {
                if self.portal_lookup.contains_key(&channel.id) {
                    self.route_portal_event(channel.id, PortalEvent::ChannelDelete);
                }
            }
            DiscordEvent::InteractionCreate(command) => match command.inner {
                interactions::SlashCommandType::Ping => {
                    // TODO: better error handling
                    // TODO: better task supervision
                    let http = self.http.clone();
                    tokio::spawn(async move {
                        let _ = command
                            .interaction
                            .create_response(
                                &http,
                                CreateInteractionResponse::Message(
                                    CreateInteractionResponseMessage::new()
                                        .ephemeral(true)
                                        .content("pong!"),
                                ),
                            )
                            .await;
                    });
                }
                interactions::SlashCommandType::LinkChannel {
                    discord_channel_id,
                    lamprey_channel_id,
                    backfill: _,
                } => {
                    // TODO: better error handling
                    // TODO: better task supervision
                    // TODO: handle backfill: true
                    let http = self.http.clone();
                    let bridge = self.bridge.clone();

                    // check if channel is already linked
                    if self.portal_lookup.contains_key(&discord_channel_id) {
                        tokio::spawn(async move {
                            let _ = command
                                .interaction
                                .create_response(
                                    &http,
                                    CreateInteractionResponse::Message(
                                        CreateInteractionResponseMessage::new()
                                            .ephemeral(true)
                                            .content("this channel is already linked"),
                                    ),
                                )
                                .await;
                        });
                        return;
                    }

                    tokio::spawn(async move {
                        // get channel
                        let discord_channel =
                            match http.get_channel(discord_channel_id).await.and_then(|ch| {
                                ch.guild()
                                    .ok_or(serenity::Error::Other("not a guild channel"))
                            }) {
                                Ok(ch) => ch,
                                Err(e) => {
                                    error!(?e, "failed to get channel");
                                    return;
                                }
                            };
                        let discord_last_id = discord_channel.last_message_id.unwrap_or_default();

                        // create webhook
                        let webhook = match discord_channel
                            .create_webhook(&http, CreateWebhook::new("bridge"))
                            .await
                        {
                            Ok(wh) => wh,
                            Err(e) => {
                                error!(?e, "failed to create webhook");
                                let _ = command
                                    .interaction
                                    .create_response(
                                        &http,
                                        CreateInteractionResponse::Message(
                                            CreateInteractionResponseMessage::new()
                                                .ephemeral(true)
                                                .content("failed to create webhook"),
                                        ),
                                    )
                                    .await;
                                return;
                            }
                        };

                        let webhook_url: url::Url = webhook
                            .url()
                            .expect("webhook url")
                            .parse()
                            .expect("invalid webhook url");

                        // send command to bridge actor
                        let _ = bridge
                            .commands
                            .send(BridgeCommand::PortalLinkRequest {
                                discord_guild_id: command.guild_id(),
                                discord_channel_id,
                                lamprey_channel_id,
                                webhook_url,
                                webhook_id: webhook.id,
                                discord_last_id,
                            })
                            .await;

                        // respond to user
                        let _ = command
                            .interaction
                            .create_response(
                                &http,
                                CreateInteractionResponse::Message(
                                    CreateInteractionResponseMessage::new()
                                        .ephemeral(true)
                                        .content("please send !accept from the lamprey side"),
                                ),
                            )
                            .await;
                    });
                }
                interactions::SlashCommandType::LinkGuild {
                    discord_guild_id,
                    lamprey_room_id,
                    backfill: _,
                    continuous,
                } => {
                    let http = self.http.clone();
                    let bridge = self.bridge.clone();
                    let discord_channel_id = command.interaction.channel_id;

                    tokio::spawn(async move {
                        let _ = bridge
                            .commands
                            .send(BridgeCommand::RealmLinkRequest {
                                discord_guild_id,
                                discord_channel_id,
                                lamprey_room_id,
                                continuous,
                            })
                            .await;

                        let _ = command
                            .interaction
                            .create_response(
                                &http,
                                CreateInteractionResponse::Message(
                                    CreateInteractionResponseMessage::new()
                                        .ephemeral(true)
                                        .content("guild link initiated"),
                                ),
                            )
                            .await;
                    });
                }
                interactions::SlashCommandType::UnlinkGuild { discord_guild_id } => {
                    let http = self.http.clone();
                    let bridge = self.bridge.clone();

                    tokio::spawn(async move {
                        let _ = bridge
                            .commands
                            .send(BridgeCommand::RealmUnlink { discord_guild_id })
                            .await;

                        let _ = command
                            .interaction
                            .create_response(
                                &http,
                                CreateInteractionResponse::Message(
                                    CreateInteractionResponseMessage::new()
                                        .ephemeral(true)
                                        .content("guild unlinked"),
                                ),
                            )
                            .await;
                    });
                }
                interactions::SlashCommandType::UnlinkChannel { discord_channel_id } => {
                    let http = self.http.clone();
                    let bridge = self.bridge.clone();

                    tokio::spawn(async move {
                        let _ = bridge
                            .commands
                            .send(BridgeCommand::PortalUnlink { discord_channel_id })
                            .await;

                        let _ = command
                            .interaction
                            .create_response(
                                &http,
                                CreateInteractionResponse::Message(
                                    CreateInteractionResponseMessage::new()
                                        .ephemeral(true)
                                        .content("channel unlinked"),
                                ),
                            )
                            .await;
                    });
                }
            },
        }
    }

    fn route_portal_event(&self, channel_id: ChannelId, event: PortalEvent) {
        info!("forwarding event to portal for channel: {:?}", channel_id);
        if let Some(portal_id) = self.portal_lookup.get(&channel_id) {
            if let Some(handle) = self.portal_handles.get(portal_id) {
                let _ = handle.events.send(Arc::new(event));
            }
        }
    }

    async fn handle_bridge_event(&mut self, event: &BridgeEvent) {
        match event {
            BridgeEvent::RealmInit(realm, handle) => {
                self.realm_handles.insert(realm.id, handle.clone());
                self.realm_data.insert(realm.id, realm.clone());
                self.spawn_realm_task(realm.id);
            }
            BridgeEvent::PortalInit(portal, handle) => {
                self.init_portal(portal, handle);
            }
            BridgeEvent::PortalCreated(portal) => {
                let handle = self.bridge.create_portal_handle(portal.id);
                self.init_portal(portal, &handle);
            }
            BridgeEvent::PortalEvent(id, event) => {
                if let Some(handle) = self.portal_handles.get(id) {
                    let _ = handle.events.send(Arc::new(event.clone()));
                }
            }
            BridgeEvent::PortalDeleted(id) => {
                self.portal_lookup.retain(|_, v| v != id);
                self.portal_handles.remove(id);
                // TODO: make sure portal tasks exit when their PortalHandle is dropped
            }
            BridgeEvent::PortalLinkResponse {
                discord_channel_id,
                accepted,
            } => {
                let msg = if *accepted {
                    "portal successfully created!"
                } else {
                    "portal request was declined (or maybe something else went wrong)"
                };
                let http = self.http.clone();
                let channel_id = *discord_channel_id;
                tokio::spawn(async move {
                    let _ = http
                        .send_message(channel_id, vec![], &CreateMessage::new().content(msg))
                        .await;
                });
            }
            BridgeEvent::RealmLinkResponse {
                discord_guild_id: _,
                discord_channel_id,
                accepted,
            } => {
                let msg = if *accepted {
                    "guild successfully linked!"
                } else {
                    "guild request was declined (or maybe something else went wrong)"
                };
                let http = self.http.clone();
                let channel_id = *discord_channel_id;
                tokio::spawn(async move {
                    let _ = http
                        .send_message(channel_id, vec![], &CreateMessage::new().content(msg))
                        .await;
                });
            }
            _ => {} // other events not relevant to Discord platform
        }
    }

    fn init_portal(&mut self, portal: &Portal, handle: &PortalHandle) {
        if let Some(discord) = &portal.discord {
            self.portal_lookup.insert(discord.channel_id, portal.id);
            if let Some(webhook_id) = discord.webhook_id {
                self.webhook_lookup.insert(webhook_id, portal.id);
            }
        }
        self.portal_handles.insert(portal.id, handle.clone());
        self.portal_data.insert(portal.id, portal.clone());
        self.spawn_portal_task(portal.id);
    }
}

async fn spawn_portal(
    id: PortalId,
    portal: Portal,
    handle: PortalHandle,
    http: Arc<serenity::all::Http>,
    cache: Arc<serenity::all::Cache>,
) -> (PortalId, Result<()>) {
    (
        id,
        spawn_portal_inner(id, portal, handle, http, cache).await,
    )
}

async fn spawn_portal_inner(
    portal_id: PortalId,
    portal: Portal,
    handle: PortalHandle,
    http: Arc<serenity::all::Http>,
    cache: Arc<serenity::all::Cache>,
) -> Result<()> {
    let mut events = handle.events.subscribe();
    let http_client = reqwest::Client::new();
    // TODO: set user-agent header for http_client?

    // TODO: backfill missed messages

    loop {
        let event = match events.recv().await {
            Ok(e) => e,
            Err(broadcast::error::RecvError::Lagged(n)) => {
                warn!(%portal_id, n, "portal event receiver lagged, skipping");
                continue;
            }
            Err(broadcast::error::RecvError::Closed) => break,
        };

        match &*event {
            PortalEvent::Typing(_) => {
                // discord doesn't have any good way of bridging typing notifications
                // NOTE: maybe i could send typing notifs through the bridge bot if anyone is typing on lamprey?
            }
            PortalEvent::MessageCreate(data) => {
                // PERF: parse after checking MessageData::Discord
                let transformer = MessageTransformer::parse(&data);

                let (msg, user, room_member, info) = match data {
                    MessageData::Lamprey {
                        message,
                        user,
                        room_member,
                        info,
                    } => (&**message, &**user, room_member.as_deref(), &**info),
                    MessageData::Discord { .. } => continue,
                };

                // PERF: don't fetch webhook every time, cache it (Webhook::from_url)
                let discord_cfg = portal.discord.as_ref().unwrap();
                let webhook_url = &discord_cfg.webhook_url;
                let webhook = serenity::all::Webhook::from_url(&http, webhook_url.as_str()).await?;

                let msg_inner = match &msg.latest_version.message_type {
                    common::v1::types::MessageType::DefaultMarkdown(m) => m,
                    _ => {
                        debug!("unsupported lamprey message type");
                        // TODO: format and send anyways?
                        continue;
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
                    if let Some(bridge_msg) = handle
                        .bridge
                        .db
                        .message_get_by_lamprey_id(portal_id, reply_id)
                        .await?
                    {
                        if let Some(discord_msg_id) = bridge_msg.discord_message_id {
                            let discord_msg = http
                                .get_message(discord_cfg.channel_id, discord_msg_id)
                                .await?;
                            let reply_content = format_discord_reply_content(&discord_msg);
                            let description = format!(
                                "**[replying to](https://canary.discord.com/channels/{}/{}/{})**\n{}",
                                discord_cfg.guild_id,
                                discord_cfg.channel_id,
                                discord_msg_id,
                                reply_content,
                            );
                            content = format!("{} {}", discord_msg.author.mention(), content);
                            embeds.push(CreateEmbed::new().description(description));
                            if let Some(att) = discord_msg.attachments.first() {
                                embeds.push(CreateEmbed::new().image(&att.url));
                            }
                        }
                    } else {
                        // TODO: handle unknown reply
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
                            if let Ok(Some(u)) = handle
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

                let sent_message = webhook.execute(&http, true, builder).await?;

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
                            portal_id,
                            source_platform: Platform::Lamprey,
                            lamprey_message_id: Some(message.id),
                            discord_message_id: Some(msg.id),
                            attachments,
                        };

                        let _ = handle
                            .bridge
                            .db
                            .message_create(portal_id, updated_message)
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
                    MessageData::Discord { .. } => continue,
                };

                // PERF: don't fetch webhook every time, cache it (Webhook::from_url)
                let discord_cfg = portal.discord.as_ref().unwrap();
                let webhook_url = &discord_cfg.webhook_url;
                let webhook = serenity::all::Webhook::from_url(&http, webhook_url.as_str()).await?;

                let Some(portal_msg) = handle
                    .bridge
                    .db
                    .message_get_by_lamprey_id(portal_id, msg.id)
                    .await?
                else {
                    continue;
                };

                let Some(message_id) = portal_msg.discord_message_id else {
                    continue;
                };

                // TODO: deduplicate code with MessageCreate handler
                let msg_inner = match &msg.latest_version.message_type {
                    common::v1::types::MessageType::DefaultMarkdown(m) => m,
                    _ => {
                        debug!("unsupported lamprey message type");
                        // TODO: format and send anyways?
                        continue;
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
                        &http,
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
                let _ = handle
                    .bridge
                    .db
                    .message_update(portal_id, updated_message)
                    .await;
            }
            PortalEvent::MessageDelete(message_id) => {
                if let Some(discord_cfg) = portal.discord.as_ref() {
                    if let Some(msg) = handle.bridge.db.message_get(*message_id).await? {
                        if let (Some(lamprey_msg_id), Some(discord_message_id)) =
                            (msg.lamprey_message_id, msg.discord_message_id)
                        {
                            let _ = http
                                .delete_message(discord_cfg.channel_id, discord_message_id, None)
                                .await;
                            let _ = handle
                                .bridge
                                .db
                                .message_delete_by_lamprey(portal_id, lamprey_msg_id)
                                .await;
                        }
                    }
                }
            }
            // TODO: implement
            // PortalEvent::ReactionCreate(_, _, _) => todo!(),
            // PortalEvent::ReactionDelete(_, _, _) => todo!(),
            // PortalEvent::ReactionDeleteEmoji(_, _) => todo!(),
            // PortalEvent::ReactionDeleteAll(_, _) => todo!(),
            _ => {}
        }
    }

    Ok(())
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

async fn spawn_realm(
    id: RealmId,
    realm: Realm,
    handle: RealmHandle,
    http: Arc<serenity::all::Http>,
    cache: Arc<serenity::all::Cache>,
) -> (RealmId, Result<()>) {
    (id, spawn_realm_inner(id, realm, handle, http, cache).await)
}

async fn spawn_realm_inner(
    realm_id: RealmId,
    realm: Realm,
    handle: RealmHandle,
    http: Arc<serenity::all::Http>,
    _cache: Arc<serenity::all::Cache>,
) -> Result<()> {
    let mut events = handle.events.subscribe();
    loop {
        let event = match events.recv().await {
            Ok(e) => e,
            Err(broadcast::error::RecvError::Lagged(n)) => {
                warn!(%realm_id, n, "realm event receiver lagged, skipping");
                continue;
            }
            Err(broadcast::error::RecvError::Closed) => break,
        };
        match &*event {
            RealmEvent::ChannelCreate(chan) => {
                if !realm.continuous {
                    continue;
                }

                match chan {
                    ChannelData::Discord { .. } => continue,
                    ChannelData::Lamprey { channel } => {
                        let guild_id = realm.discord.as_ref().unwrap().guild_id;
                        let http = http.clone();
                        let bridge = handle.bridge.clone();

                        let create_channel =
                            CreateChannel::new(channel.name.clone()).kind(match channel.ty {
                                lamprey::ChannelType::Text => ChannelType::Text,
                                _ => continue,
                            });

                        // TODO: run below code in a tokio task instead of blocking loop

                        // TODO: add audit log reason
                        let discord_channel =
                            match http.create_channel(guild_id, &create_channel, None).await {
                                Ok(ch) => ch,
                                Err(e) => {
                                    error!(?e, "failed to create discord channel");
                                    continue;
                                }
                            };

                        // TODO: deduplicate this code with `/link`
                        let webhook = match discord_channel
                            .create_webhook(&http, CreateWebhook::new("bridge"))
                            .await
                        {
                            Ok(wh) => wh,
                            Err(e) => {
                                error!(?e, "failed to create webhook");
                                continue;
                            }
                        };

                        let webhook_url = webhook
                            .url()
                            .expect("webhook url")
                            .parse()
                            .expect("invalid webhook url");

                        let portal_id = PortalId::new();
                        let portal = Portal {
                            id: portal_id,
                            realm_id: Some(realm_id),
                            lamprey: Some(PortalLamprey {
                                channel_id: channel.id,
                                room_id: channel.room_id.unwrap(),
                                last_id: channel.last_message_id.unwrap_or_default(),
                            }),
                            discord: Some(PortalDiscord {
                                guild_id,
                                parent_id: None,
                                channel_id: discord_channel.id,
                                webhook_url,
                                webhook_id: Some(webhook.id),
                                last_id: discord_channel.last_message_id.unwrap_or_default(),
                            }),
                        };

                        if bridge.db.portal_create(portal.clone()).await.is_ok() {
                            let _ = bridge
                                .events
                                .send(Arc::new(BridgeEvent::PortalCreated(portal)));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
