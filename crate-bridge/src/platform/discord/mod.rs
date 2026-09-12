use std::collections::HashMap;
use std::sync::Arc;

use serenity::all::{
    CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage, CreateWebhook,
    GatewayIntents,
};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinSet;
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
    bridge_old::{BridgeEvent, BridgeHandle, PortalEvent},
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
    OnlineStatus, Presence, ReactionType, RoleId, User, UserId, WebhookId,
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
        self.portal_tasks.spawn(DiscordPortal::spawn(
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
        self.realm_tasks
            .spawn(crate::platform::discord::realm::DiscordRealm::spawn(
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
            DiscordEvent::ReactionAdd(reaction) => {
                if let Some(portal_id) = self.portal_lookup.get(&reaction.channel_id) {
                    if let Ok(Some(msg)) = self
                        .bridge
                        .db
                        .message_get_by_discord_id(*portal_id, reaction.message_id)
                        .await
                    {
                        if let Ok(Some(user)) = self
                            .bridge
                            .db
                            .puppet_get_by_discord_id(reaction.user_id.unwrap().to_string())
                            .await
                        {
                            self.route_portal_event(
                                reaction.channel_id,
                                PortalEvent::ReactionCreate(
                                    msg.id,
                                    bridge::ReactionKey::Discord(reaction.emoji),
                                    user,
                                ),
                            );
                        }
                    }
                }
            }
            DiscordEvent::ReactionRemove(reaction) => {
                if let Some(portal_id) = self.portal_lookup.get(&reaction.channel_id) {
                    if let Ok(Some(msg)) = self
                        .bridge
                        .db
                        .message_get_by_discord_id(*portal_id, reaction.message_id)
                        .await
                    {
                        if let Some(user_id) = reaction.user_id {
                            if let Ok(Some(user)) = self
                                .bridge
                                .db
                                .puppet_get_by_discord_id(user_id.to_string())
                                .await
                            {
                                self.route_portal_event(
                                    reaction.channel_id,
                                    PortalEvent::ReactionDelete(
                                        msg.id,
                                        bridge::ReactionKey::Discord(reaction.emoji),
                                        user,
                                    ),
                                );
                            }
                        }
                    }
                }
            }
            DiscordEvent::ReactionRemoveAll(channel_id, message_id) => {
                if let Some(portal_id) = self.portal_lookup.get(&channel_id) {
                    if let Ok(Some(msg)) = self
                        .bridge
                        .db
                        .message_get_by_discord_id(*portal_id, message_id)
                        .await
                    {
                        self.route_portal_event(channel_id, PortalEvent::ReactionDeleteAll(msg.id));
                    }
                }
            }
            DiscordEvent::ReactionRemoveEmoji(reaction) => {
                if let Some(portal_id) = self.portal_lookup.get(&reaction.channel_id) {
                    if let Ok(Some(msg)) = self
                        .bridge
                        .db
                        .message_get_by_discord_id(*portal_id, reaction.message_id)
                        .await
                    {
                        self.route_portal_event(
                            reaction.channel_id,
                            PortalEvent::ReactionDeleteKey(
                                msg.id,
                                bridge::ReactionKey::Discord(reaction.emoji),
                            ),
                        );
                    }
                }
            }
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
