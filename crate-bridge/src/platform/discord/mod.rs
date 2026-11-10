use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use serenity::all::{
    CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage, CreateWebhook,
    GatewayIntents,
};
use tokio::sync::{mpsc, oneshot};
use tokio::task::{AbortHandle, JoinSet};
use tracing::{debug, error, info, warn};

use crate::actor::bridge::BridgeCommand;
use crate::bridge_old as bridge;
use crate::bridge_old::{
    MessageData, PlatformHandle, Portal, PortalHandle, PortalId, Realm, RealmEvent, RealmHandle,
    RealmId,
};
use crate::config::Config;
use crate::platform::discord::events::DiscordEvent;
use crate::platform::discord::portal::DiscordPortal;
use crate::prelude::*;
use crate::types::ChannelData;
use crate::{
    bridge_old::{BridgeEvent, BridgeHandle, PortalEvent, PortalEventEnvelope, ThreadId},
    config::DiscordConfig,
};

mod events;
mod interactions;
mod portal;
mod realm;

// re export discord (serenity) types
pub use serenity::all::{
    Activity, ActivityType, Attachment, AttachmentId, Channel, ChannelId, ChannelType,
    CreateAllowedMentions, CreateEmbed, Embed, GuildChannel, GuildId, Message, MessageId,
    MessageType, OnlineStatus, Presence, ReactionType, RoleId, User, UserId, WebhookId,
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
    portal_aborts: HashMap<PortalId, AbortHandle>,
    realm_aborts: HashMap<RealmId, AbortHandle>,
    portal_handles: HashMap<PortalId, PortalHandle>,
    portal_lookup: HashMap<ChannelId, PortalId>,
    portal_data: HashMap<PortalId, Portal>,
    realm_handles: HashMap<RealmId, RealmHandle>,
    realm_lookup: HashMap<ChannelId, RealmId>,
    realm_data: HashMap<RealmId, Realm>,
    webhook_lookup: HashMap<serenity::all::WebhookId, PortalId>,
    http: Arc<serenity::all::Http>,
    cache: Arc<serenity::all::Cache>,
    pending_threads: HashMap<ChannelId, Vec<GuildChannel>>,
    thread_parents: HashMap<ChannelId, ChannelId>,
    channels_creating: Arc<Mutex<HashSet<ChannelId>>>,
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
            portal_aborts: HashMap::new(),
            realm_aborts: HashMap::new(),
            portal_handles: HashMap::new(),
            portal_lookup: HashMap::new(),
            portal_data: HashMap::new(),
            realm_handles: HashMap::new(),
            realm_lookup: HashMap::new(),
            realm_data: HashMap::new(),
            webhook_lookup: HashMap::new(),
            http,
            cache,
            pending_threads: HashMap::new(),
            thread_parents: HashMap::new(),
            channels_creating: Arc::new(Mutex::new(HashSet::new())),
        };
        me.start(client, ready_tx).await?;

        Ok(())
    }

    fn spawn_portal_task(&mut self, portal_id: PortalId) {
        let portal = self.portal_data.get(&portal_id).unwrap().clone();
        let handle = self.portal_handles.get(&portal_id).unwrap().clone();
        let abort = self.portal_tasks.spawn(DiscordPortal::spawn(
            portal_id,
            portal,
            handle,
            self.http.clone(),
            self.cache.clone(),
        ));
        self.portal_aborts.insert(portal_id, abort);
    }

    fn spawn_realm_task(&mut self, realm_id: RealmId) {
        let realm = self.realm_data.get(&realm_id).unwrap().clone();
        let handle = self.realm_handles.get(&realm_id).unwrap().clone();
        let abort = self
            .realm_tasks
            .spawn(crate::platform::discord::realm::DiscordRealm::spawn(
                realm_id,
                realm,
                handle,
                self.http.clone(),
                self.cache.clone(),
            ));
        self.realm_aborts.insert(realm_id, abort);
    }

    /// stop and forget a portal, deleting the webhook we created for it
    fn teardown_portal(&mut self, portal_id: PortalId) {
        if let Some(abort) = self.portal_aborts.remove(&portal_id) {
            abort.abort();
        }
        let portal = self.portal_data.remove(&portal_id);
        self.portal_handles.remove(&portal_id);
        self.portal_lookup.retain(|_, v| *v != portal_id);
        self.webhook_lookup.retain(|_, v| *v != portal_id);

        let Some(discord) = portal.and_then(|portal| portal.discord) else {
            return;
        };
        if discord.webhook_id.is_none() {
            return;
        }

        let http = self.http.clone();
        let url = discord.webhook_url;
        tokio::spawn(async move {
            match serenity::all::Webhook::from_url(&http, url.as_str()).await {
                Ok(webhook) => {
                    if let Err(e) = webhook.delete(&http).await {
                        warn!(?e, "failed to delete webhook for unlinked portal");
                    }
                }
                Err(e) => warn!(?e, "failed to resolve webhook for unlinked portal"),
            }
        });
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
                            self.portal_aborts.remove(&portal_id);
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
                            self.realm_aborts.remove(&realm_id);
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

                // sometimes we receive thread create system messages before the
                // actual thread create event
                if message.kind == MessageType::ThreadCreated {
                    if let Some(thread) = message.thread.clone() {
                        self.route_thread_create(thread);
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
                let new_message = match new {
                    Some(msg) => msg,
                    None => match self.http.get_message(event.channel_id, event.id).await {
                        Ok(msg) => msg,
                        Err(e) => {
                            warn!(
                                message_id = %event.id,
                                channel_id = %event.channel_id,
                                "failed to fetch updated message: {e:?}"
                            );
                            return;
                        }
                    },
                };

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
            DiscordEvent::TypingStart(event) => {
                let discord_id = event.user_id.get().to_string();
                if let Ok(Some(user)) = self.bridge.db.puppet_get_by_discord_id(discord_id).await {
                    self.route_portal_event(event.channel_id, PortalEvent::Typing(user));
                }
            }
            DiscordEvent::MessageDelete(channel_id, message_id) => {
                if let Some((portal_id, thread)) = self.lookup_portal(channel_id) {
                    if let Ok(Some(msg)) = self
                        .bridge
                        .db
                        .message_get_by_discord_id(portal_id, message_id)
                        .await
                    {
                        self.send_portal_event(
                            portal_id,
                            PortalEvent::MessageDelete(msg.id),
                            thread,
                        );
                    }
                }
            }

            DiscordEvent::PresenceUpdate(presence) => {
                let _ = self
                    .bridge
                    .events
                    .send(Arc::new(BridgeEvent::PresenceUpdate(presence)));
            }
            DiscordEvent::GuildMemberUpdate(event) => {
                let discord_user_id = event.user.id.to_string();
                if let Ok(Some(puppet)) = self
                    .bridge
                    .db
                    .puppet_get_by_discord_id(discord_user_id)
                    .await
                {
                    let guild_id = event.guild_id;
                    let Some(realm_id) = self.realm_data.iter().find_map(|(id, realm)| {
                        if let Some(r_discord) = &realm.discord {
                            if r_discord.guild_id == guild_id {
                                return Some(*id);
                            }
                        }
                        None
                    }) else {
                        return;
                    };

                    let nickname = event.nick;
                    let member = bridge_old::RealmMember {
                        realm_id,
                        user_lamprey_id: puppet.lamprey_id,
                        nickname,
                    };

                    let _ = self.bridge.db.realm_member_upsert(member.clone()).await;

                    if let Some(handle) = self.realm_handles.get(&realm_id) {
                        let _ = handle
                            .events
                            .send(Arc::new(RealmEvent::MemberUpdate(member)));
                    }
                }
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

                let Some(handle) = self.realm_handles.get(&realm_id).cloned() else {
                    return;
                };
                let http = self.http.clone();
                let channels_creating = self.channels_creating.clone();

                tokio::spawn(async move {
                    Self::create_realm_channel(&handle, &http, &channels_creating, channel).await;
                });
            }
            DiscordEvent::ChannelDelete(channel) => {
                if self.lookup_portal(channel.id).is_some() {
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
            DiscordEvent::ReactionAdd(reaction) => {
                if let Some((portal_id, thread)) = self.lookup_portal(reaction.channel_id) {
                    if let Ok(Some(msg)) = self
                        .bridge
                        .db
                        .message_get_by_discord_id(portal_id, reaction.message_id)
                        .await
                    {
                        if let Ok(Some(user)) = self
                            .bridge
                            .db
                            .puppet_get_by_discord_id(reaction.user_id.unwrap().to_string())
                            .await
                        {
                            self.send_portal_event(
                                portal_id,
                                PortalEvent::ReactionCreate(
                                    msg.id,
                                    bridge::ReactionKey::Discord(reaction.emoji),
                                    user,
                                ),
                                thread,
                            );
                        }
                    }
                }
            }
            DiscordEvent::ReactionRemove(reaction) => {
                if let Some((portal_id, thread)) = self.lookup_portal(reaction.channel_id) {
                    if let Ok(Some(msg)) = self
                        .bridge
                        .db
                        .message_get_by_discord_id(portal_id, reaction.message_id)
                        .await
                    {
                        if let Some(user_id) = reaction.user_id {
                            if let Ok(Some(user)) = self
                                .bridge
                                .db
                                .puppet_get_by_discord_id(user_id.to_string())
                                .await
                            {
                                self.send_portal_event(
                                    portal_id,
                                    PortalEvent::ReactionDelete(
                                        msg.id,
                                        bridge::ReactionKey::Discord(reaction.emoji),
                                        user,
                                    ),
                                    thread,
                                );
                            }
                        }
                    }
                }
            }
            DiscordEvent::ReactionRemoveAll(channel_id, message_id) => {
                if let Some((portal_id, thread)) = self.lookup_portal(channel_id) {
                    if let Ok(Some(msg)) = self
                        .bridge
                        .db
                        .message_get_by_discord_id(portal_id, message_id)
                        .await
                    {
                        self.send_portal_event(
                            portal_id,
                            PortalEvent::ReactionDeleteAll(msg.id),
                            thread,
                        );
                    }
                }
            }
            DiscordEvent::ReactionRemoveEmoji(reaction) => {
                if let Some((portal_id, thread)) = self.lookup_portal(reaction.channel_id) {
                    if let Ok(Some(msg)) = self
                        .bridge
                        .db
                        .message_get_by_discord_id(portal_id, reaction.message_id)
                        .await
                    {
                        self.send_portal_event(
                            portal_id,
                            PortalEvent::ReactionDeleteKey(
                                msg.id,
                                bridge::ReactionKey::Discord(reaction.emoji),
                            ),
                            thread,
                        );
                    }
                }
            }
            DiscordEvent::ThreadCreate(thread) => {
                self.route_thread_create(thread);
            }
            DiscordEvent::ThreadUpdate(thread) => {
                self.route_portal_event(
                    thread.id,
                    PortalEvent::ChannelUpdate(ChannelData::Discord {
                        channel: Box::new(thread),
                        webhook: None,
                    }),
                );
            }
            DiscordEvent::ThreadDelete(thread_id) => {
                self.thread_parents.remove(&thread_id);
                if self.lookup_portal(thread_id).is_some() {
                    self.route_portal_event(thread_id, PortalEvent::ChannelDelete);
                }
            }
            DiscordEvent::ThreadListSync(event) => {
                for thread in event.threads {
                    if self.portal_lookup.contains_key(&thread.id) {
                        continue;
                    }
                    self.route_thread_create(thread);
                }
            }
            DiscordEvent::ThreadMembersUpdate(event) => {
                debug!(thread_id = %event.id, "ignoring thread members update");
            }
            DiscordEvent::GuildCreate(guild) => {
                self.handle_guild_create(guild).await;
            }
            _ => debug!("unhandled discord event"),
        }
    }

    async fn handle_guild_create(&mut self, guild: serenity::all::Guild) {
        let realm_id = self.realm_data.iter().find_map(|(id, realm)| {
            if let Some(r_discord) = &realm.discord {
                if r_discord.guild_id == guild.id && realm.continuous {
                    return Some(*id);
                }
            }
            None
        });

        for channel in guild.channels.values().chain(guild.threads.iter()) {
            if self.portal_lookup.contains_key(&channel.id) {
                continue;
            }

            if self
                .bridge
                .db
                .portal_get_by_discord_channel(channel.id.to_string())
                .await
                .is_ok_and(|portal| portal.is_some())
            {
                continue;
            }

            match channel.kind {
                ChannelType::PublicThread
                | ChannelType::PrivateThread
                | ChannelType::NewsThread => {
                    self.route_thread_create(channel.clone());
                }
                ChannelType::Text | ChannelType::News => {
                    let Some(realm_id) = realm_id else {
                        continue;
                    };
                    let Some(handle) = self.realm_handles.get(&realm_id).cloned() else {
                        continue;
                    };
                    let channels_creating = self.channels_creating.clone();
                    Self::create_realm_channel(
                        &handle,
                        &self.http,
                        &channels_creating,
                        channel.clone(),
                    )
                    .await;
                }
                _ => {}
            }
        }
    }

    async fn create_realm_channel(
        handle: &RealmHandle,
        http: &Arc<serenity::all::Http>,
        channels_creating: &Arc<Mutex<HashSet<ChannelId>>>,
        channel: serenity::all::GuildChannel,
    ) {
        let channel_id = channel.id;

        {
            let mut creating = channels_creating.lock().unwrap();
            if !creating.insert(channel_id) {
                debug!(%channel_id, "channel creation already in progress");
                return;
            }
        }

        if handle
            .bridge
            .db
            .portal_get_by_discord_channel(channel_id.to_string())
            .await
            .is_ok_and(|portal| portal.is_some())
        {
            channels_creating.lock().unwrap().remove(&channel_id);
            return;
        }

        let is_text = matches!(channel.kind, ChannelType::Text | ChannelType::News);
        let channel_data = if is_text {
            let webhook = match channel
                .create_webhook(http, CreateWebhook::new("bridge"))
                .await
            {
                Ok(wh) => wh,
                Err(e) => {
                    error!(?e, "failed to create webhook");
                    channels_creating.lock().unwrap().remove(&channel_id);
                    return;
                }
            };

            let webhook_url: url::Url = webhook
                .url()
                .expect("webhook url")
                .parse()
                .expect("invalid webhook url");

            ChannelData::Discord {
                channel: Box::new(channel),
                webhook: Some((webhook.id, webhook_url)),
            }
        } else {
            ChannelData::Discord {
                channel: Box::new(channel),
                webhook: None,
            }
        };

        if handle
            .events
            .send(Arc::new(RealmEvent::ChannelCreate(channel_data)))
            .is_err()
        {
            channels_creating.lock().unwrap().remove(&channel_id);
        }
    }

    fn route_thread_create(&mut self, thread: GuildChannel) {
        let Some(parent_id) = thread.parent_id else {
            return;
        };
        self.thread_parents.insert(thread.id, parent_id);

        if self.portal_lookup.contains_key(&thread.id) {
            return;
        }

        if self.portal_lookup.contains_key(&parent_id) {
            self.route_portal_event(
                parent_id,
                PortalEvent::ThreadCreate(ChannelData::Discord {
                    channel: Box::new(thread),
                    webhook: None,
                }),
            );
        } else {
            debug!(thread_id = %thread.id, %parent_id, "deferring thread until parent portal exists");
            self.pending_threads
                .entry(parent_id)
                .or_default()
                .push(thread);
        }
    }

    /// Resolve the portal that should handle an event. Threads without a portal
    /// of their own fall back to the portal of their parent channel; the thread
    /// id is returned so the portal can target the thread itself.
    fn lookup_portal(&self, channel_id: ChannelId) -> Option<(PortalId, Option<ThreadId>)> {
        if let Some(portal_id) = self.portal_lookup.get(&channel_id) {
            return Some((*portal_id, None));
        }

        let parent_id = self.thread_parents.get(&channel_id).copied().or_else(|| {
            self.cache
                .guilds()
                .into_iter()
                .filter_map(|guild_id| self.cache.guild(guild_id))
                .find_map(|guild| {
                    guild
                        .threads
                        .iter()
                        .find(|thread| thread.id == channel_id)
                        .and_then(|thread| thread.parent_id)
                })
        });

        let Some(parent_id) = parent_id else {
            debug!(%channel_id, "no portal found for channel");
            return None;
        };

        let Some(portal_id) = self.portal_lookup.get(&parent_id) else {
            debug!(%channel_id, %parent_id, "no portal found for thread or its parent");
            return None;
        };

        debug!(%channel_id, %parent_id, "routing thread event through parent portal");
        Some((*portal_id, Some(ThreadId::Discord(channel_id))))
    }

    fn send_portal_event(&self, portal_id: PortalId, event: PortalEvent, thread: Option<ThreadId>) {
        let Some(handle) = self.portal_handles.get(&portal_id) else {
            debug!(%portal_id, "no portal handle, dropping event");
            return;
        };

        if handle
            .events
            .send(Arc::new(PortalEventEnvelope { event, thread }))
            .is_err()
        {
            debug!(%portal_id, "no portal event receivers, dropping event");
        }
    }

    fn route_portal_event(&self, channel_id: ChannelId, event: PortalEvent) {
        info!("forwarding event to portal for channel: {:?}", channel_id);
        match self.lookup_portal(channel_id) {
            Some((portal_id, thread)) => self.send_portal_event(portal_id, event, thread),
            None => debug!(%channel_id, ?event, "dropping event without a portal"),
        }
    }

    async fn handle_bridge_event(&mut self, event: &BridgeEvent) {
        match event {
            BridgeEvent::RealmInit(realm, handle) => {
                self.realm_handles.insert(realm.id, handle.clone());
                self.realm_data.insert(realm.id, realm.clone());
                if !self.realm_aborts.contains_key(&realm.id) {
                    self.spawn_realm_task(realm.id);
                }
            }
            BridgeEvent::RealmDeleted(id) => {
                if let Some(abort) = self.realm_aborts.remove(id) {
                    abort.abort();
                }
                self.realm_handles.remove(id);
                self.realm_data.remove(id);
                self.realm_lookup.retain(|_, v| v != id);

                let orphans: Vec<PortalId> = self
                    .portal_data
                    .iter()
                    .filter(|(_, portal)| portal.realm_id == Some(*id))
                    .map(|(portal_id, _)| *portal_id)
                    .collect();
                for portal_id in orphans {
                    self.teardown_portal(portal_id);
                }
            }
            BridgeEvent::PortalInit(portal, handle) => {
                self.init_portal(portal, handle);
            }
            BridgeEvent::PortalCreated(portal, handle) => {
                self.init_portal(portal, handle);
            }
            BridgeEvent::PortalEvent(id, event) => {
                self.send_portal_event(*id, event.clone(), None);
            }
            BridgeEvent::PortalDeleted(id) => {
                self.teardown_portal(*id);
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
            self.channels_creating
                .lock()
                .unwrap()
                .remove(&discord.channel_id);
            if let Some(webhook_id) = discord.webhook_id {
                self.webhook_lookup.insert(webhook_id, portal.id);
            }
        }
        self.portal_handles.insert(portal.id, handle.clone());
        self.portal_data.insert(portal.id, portal.clone());
        self.spawn_portal_task(portal.id);

        if let Some(discord) = &portal.discord {
            if let Some(threads) = self.pending_threads.remove(&discord.channel_id) {
                for thread in threads {
                    if self.portal_lookup.contains_key(&thread.id) {
                        continue;
                    }
                    self.route_thread_create(thread);
                }
            }
        }
    }
}
