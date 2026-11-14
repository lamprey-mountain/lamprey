use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::{mpsc, oneshot};
use tokio::task::{AbortHandle, JoinSet};
use tracing::{debug, error, info, warn};
use twilight_cache_inmemory::InMemoryCache;
use twilight_gateway::StreamExt as _;
use twilight_gateway::{
    ConfigBuilder, Event as GatewayEvent, EventTypeFlags, Intents, Shard, ShardId,
};
use twilight_http::Client as HttpClient;
use twilight_model::application::interaction::Interaction;
use twilight_model::channel::Channel;
use twilight_model::channel::message::MessageFlags;
use twilight_model::gateway::payload::incoming::GuildCreate;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_model::id::Id;

use crate::actor::bridge::BridgeCommand;
use crate::bridge_old as bridge;
use crate::bridge_old::{BridgeEvent, BridgeHandle, PortalEvent, PortalEventEnvelope, ThreadId};
use crate::bridge_old::{
    MessageData, PlatformHandle, Portal, PortalHandle, PortalId, Realm, RealmEvent, RealmHandle,
    RealmId,
};
use crate::config::{Config, DiscordConfig};
use crate::platform::discord::interactions::{get_commands, parse_interaction};
use crate::platform::discord::portal::DiscordPortal;
use crate::prelude::*;
use crate::types::{ChannelData, Platform};

mod interactions;
mod portal;
mod realm;

// re export discord (twilight) types
pub use twilight_model::channel::Attachment;
pub use twilight_model::channel::ChannelType;
pub use twilight_model::channel::message::{
    AllowedMentions as CreateAllowedMentions, EmojiReactionType as ReactionType, MentionType,
    Message, MessageReferenceType, MessageType,
};
pub use twilight_model::gateway::presence::{
    Activity, ActivityType, Presence, Status as OnlineStatus,
};
pub use twilight_model::guild::{Guild, Permissions};
pub use twilight_model::id::marker::{
    AttachmentMarker, ChannelMarker, GuildMarker, MessageMarker, RoleMarker, UserMarker,
    WebhookMarker,
};
pub use twilight_model::user::User;

pub type GuildId = Id<GuildMarker>;
pub type ChannelId = Id<ChannelMarker>;
pub type MessageId = Id<MessageMarker>;
pub type UserId = Id<UserMarker>;
pub type RoleId = Id<RoleMarker>;
pub type WebhookId = Id<WebhookMarker>;
pub type AttachmentId = Id<AttachmentMarker>;
pub type GuildChannel = Channel;

/// smallest possible non-zero [`MessageId`], used when a channel or thread has no
/// messages yet (`Id` cannot represent zero)
pub const MIN_MESSAGE_ID: MessageId = Id::new(1);

const RECONNECT_MIN_DELAY: Duration = Duration::from_secs(1);
const RECONNECT_MAX_DELAY: Duration = Duration::from_secs(60);
const DISCORD_API_BASE: &str = "https://discord.com/api";

pub fn spawn(bridge: BridgeHandle, config_full: Config, config: DiscordConfig) -> PlatformHandle {
    let (tx, rx) = oneshot::channel();
    let task = tokio::spawn(Discord::connect(bridge, config_full, config, tx));
    PlatformHandle {
        name: "discord",
        ready: rx,
        task,
    }
}

/// a message update that is being refetched before being bridged
struct PendingUpdate {
    /// bumped whenever a newer update for the same message arrives
    generation: u64,
    message: Message,
}

struct Discord {
    bridge: BridgeHandle,
    disable_slash_command_permission_checks: bool,
    rx: mpsc::Receiver<GatewayEvent>,
    updates_rx: mpsc::UnboundedReceiver<(MessageId, u64, Option<Message>)>,
    updates_tx: mpsc::UnboundedSender<(MessageId, u64, Option<Message>)>,
    pending_updates: HashMap<MessageId, PendingUpdate>,
    update_generation: u64,
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
    webhook_lookup: HashMap<WebhookId, PortalId>,
    http: Arc<HttpClient>,
    cache: Arc<InMemoryCache>,
    pending_threads: HashMap<ChannelId, Vec<Channel>>,
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
        let token = config
            .token
            .load()
            .expect("failed to load token")
            .to_string();

        let http = Arc::new(HttpClient::new(token.clone()));
        let cache = Arc::new(InMemoryCache::builder().message_cache_size(0).build());

        let (tx, rx) = mpsc::channel(1024);
        tokio::spawn(run_gateway(token, tx));

        let (updates_tx, updates_rx) = mpsc::unbounded_channel();

        let me = Self {
            bridge,
            disable_slash_command_permission_checks: config_full
                .disable_discord_slash_command_permission_checks,
            rx,
            updates_rx,
            updates_tx,
            pending_updates: HashMap::new(),
            update_generation: 0,
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
        me.start(ready_tx).await?;

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
        let Some(webhook_id) = discord.webhook_id else {
            return;
        };

        let http = self.http.clone();
        tokio::spawn(async move {
            if let Err(e) = http.delete_webhook(webhook_id).await {
                warn!(?e, "failed to delete webhook for unlinked portal");
            }
        });
    }

    async fn start(mut self, ready_tx: oneshot::Sender<()>) -> Result<()> {
        let mut bridge_events = self.bridge.events.subscribe();
        let _ = ready_tx.send(());

        loop {
            tokio::select! {
                Ok(event) = bridge_events.recv() => {
                    self.handle_bridge_event(&event).await;
                }
                Some(event) = self.rx.recv() => {
                    self.handle_discord_event(event).await;
                }
                Some((id, generation, fetched)) = self.updates_rx.recv() => {
                    self.handle_fetched_update(id, generation, fetched);
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

    async fn handle_discord_event(&mut self, event: GatewayEvent) {
        self.cache.update(&event);

        match event {
            GatewayEvent::Ready(ready) => {
                info!("discord ready {}", ready.user.name);
                if let Err(e) = self
                    .http
                    .interaction(ready.application.id)
                    .set_global_commands(&get_commands(
                        self.disable_slash_command_permission_checks,
                    ))
                    .await
                {
                    error!("error while registering commands: {e:?}");
                }
            }
            GatewayEvent::MessageCreate(event) => {
                let message = (*event).message;

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
            GatewayEvent::MessageUpdate(event) => {
                let update = (*event).0;

                if let Some(webhook_id) = update.webhook_id {
                    if self.webhook_lookup.contains_key(&webhook_id) {
                        return;
                    }
                }

                // edits of messages that we sent to discord are echoes of our own
                // edits (eg. from unfurling), so never bridge them back
                if let Some((portal_id, _)) = self.lookup_portal(update.channel_id) {
                    if let Ok(Some(bridged)) = self
                        .bridge
                        .db
                        .message_get_by_discord_id(portal_id, update.id)
                        .await
                    {
                        if bridged.source_platform == Platform::Lamprey {
                            return;
                        }
                    }
                }

                // update payloads can be partial, so refetch the full message.
                // this must not block the gateway loop: discord rate limits every
                // single-message fetch per channel
                self.queue_message_update(update);
            }
            GatewayEvent::TypingStart(event) => {
                let discord_id = event.user_id.to_string();
                if let Ok(Some(user)) = self.bridge.db.puppet_get_by_discord_id(discord_id).await {
                    self.route_portal_event(event.channel_id, PortalEvent::Typing(user));
                }
            }
            GatewayEvent::MessageDelete(event) => {
                // don't bridge an edit that was queued before the delete
                self.pending_updates.remove(&event.id);

                if let Some((portal_id, thread)) = self.lookup_portal(event.channel_id) {
                    if let Ok(Some(msg)) = self
                        .bridge
                        .db
                        .message_get_by_discord_id(portal_id, event.id)
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
            GatewayEvent::MessageDeleteBulk(event) => {
                let Some((portal_id, thread)) = self.lookup_portal(event.channel_id) else {
                    return;
                };
                for message_id in event.ids {
                    self.pending_updates.remove(&message_id);

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

            GatewayEvent::PresenceUpdate(presence) => {
                let _ = self
                    .bridge
                    .events
                    .send(Arc::new(BridgeEvent::PresenceUpdate((*presence).0)));
            }
            GatewayEvent::MemberUpdate(event) => {
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

                    let nickname = event.nick.clone();
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
            GatewayEvent::ChannelCreate(event) => {
                let channel = (*event).0;
                let has_continuous = self.realm_data.values().any(|r| r.continuous);
                if !has_continuous {
                    return;
                }

                let is_supported = matches!(
                    channel.kind,
                    ChannelType::GuildText
                        | ChannelType::GuildAnnouncement
                        | ChannelType::GuildCategory
                );
                if !is_supported {
                    return;
                }

                let Some(guild_id) = channel.guild_id else {
                    return;
                };
                let Some(realm_id) = self.realm_data.iter().find_map(|(id, realm)| {
                    if let Some(r_discord) = &realm.discord {
                        if r_discord.guild_id == guild_id && realm.continuous {
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
            GatewayEvent::ChannelDelete(event) => {
                let channel = (*event).0;
                if self.lookup_portal(channel.id).is_some() {
                    self.route_portal_event(channel.id, PortalEvent::ChannelDelete);
                }
            }
            GatewayEvent::InteractionCreate(event) => {
                let interactions::SlashCommand {
                    interaction, inner, ..
                } = match parse_interaction((*event).0) {
                    Ok(command) => command,
                    Err(e) => {
                        error!("error parsing interaction: {e:?}");
                        return;
                    }
                };
                match inner {
                    interactions::SlashCommandType::Ping => {
                        // TODO: better error handling
                        // TODO: better task supervision
                        let http = self.http.clone();
                        tokio::spawn(async move {
                            let _ = respond(&http, &interaction, "pong!").await;
                        });
                    }
                    interactions::SlashCommandType::LinkChannel {
                        discord_channel_id,
                        lamprey_channel_id,
                        backfill: _,
                    } => {
                        // TODO: better error handling
                        // TODO: better task supervision
                        // NOTE: portals always backfill from their last known message, so
                        // the `backfill` option doesn't need any extra handling
                        let http = self.http.clone();
                        let bridge = self.bridge.clone();
                        let Some(guild_id) = interaction.guild_id else {
                            tokio::spawn(async move {
                                let _ = respond(
                                    &http,
                                    &interaction,
                                    "this command can only be used in a guild",
                                )
                                .await;
                            });
                            return;
                        };

                        // check if channel is already linked
                        if self.portal_lookup.contains_key(&discord_channel_id) {
                            tokio::spawn(async move {
                                let _ =
                                    respond(&http, &interaction, "this channel is already linked")
                                        .await;
                            });
                            return;
                        }

                        tokio::spawn(async move {
                            // get channel
                            let discord_channel = match http.channel(discord_channel_id).await {
                                Ok(r) => match r.model().await {
                                    Ok(ch) => ch,
                                    Err(e) => {
                                        error!(?e, "failed to get channel");
                                        return;
                                    }
                                },
                                Err(e) => {
                                    error!(?e, "failed to get channel");
                                    return;
                                }
                            };
                            let discord_last_id = discord_channel
                                .last_message_id
                                .map(|id| id.cast())
                                .unwrap_or(MIN_MESSAGE_ID);

                            // create webhook
                            let webhook = match http
                                .create_webhook(discord_channel_id, "bridge")
                                .await
                            {
                                Ok(r) => match r.model().await {
                                    Ok(wh) => wh,
                                    Err(e) => {
                                        error!(?e, "failed to create webhook");
                                        let _ = respond(
                                            &http,
                                            &interaction,
                                            "failed to create webhook",
                                        )
                                        .await;
                                        return;
                                    }
                                },
                                Err(e) => {
                                    error!(?e, "failed to create webhook");
                                    let _ =
                                        respond(&http, &interaction, "failed to create webhook")
                                            .await;
                                    return;
                                }
                            };

                            let Some(webhook_token) = webhook.token.as_deref() else {
                                error!("webhook response is missing its token");
                                let _ =
                                    respond(&http, &interaction, "failed to create webhook").await;
                                return;
                            };

                            let webhook_url = webhook_url(webhook.id, webhook_token);

                            // send command to bridge actor
                            let _ = bridge
                                .commands
                                .send(BridgeCommand::PortalLinkRequest {
                                    discord_guild_id: guild_id,
                                    discord_channel_id,
                                    lamprey_channel_id,
                                    webhook_url,
                                    webhook_id: webhook.id,
                                    discord_last_id,
                                })
                                .await;

                            // respond to user
                            let _ = respond(
                                &http,
                                &interaction,
                                "please send !accept from the lamprey side",
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
                        let channel_id = interaction.channel.as_ref().map(|channel| channel.id);
                        let Some(discord_channel_id) = channel_id else {
                            tokio::spawn(async move {
                                let _ = respond(
                                    &http,
                                    &interaction,
                                    "this command can only be used in a channel",
                                )
                                .await;
                            });
                            return;
                        };

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

                            let _ = respond(&http, &interaction, "guild link initiated").await;
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

                            let _ = respond(&http, &interaction, "guild unlinked").await;
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

                            let _ = respond(&http, &interaction, "channel unlinked").await;
                        });
                    }
                }
            }
            GatewayEvent::ReactionAdd(event) => {
                let reaction = (*event).0;
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
                            .puppet_get_by_discord_id(reaction.user_id.to_string())
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
            GatewayEvent::ReactionRemove(event) => {
                let reaction = (*event).0;
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
                            .puppet_get_by_discord_id(reaction.user_id.to_string())
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
            GatewayEvent::ReactionRemoveAll(event) => {
                if let Some((portal_id, thread)) = self.lookup_portal(event.channel_id) {
                    if let Ok(Some(msg)) = self
                        .bridge
                        .db
                        .message_get_by_discord_id(portal_id, event.message_id)
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
            GatewayEvent::ReactionRemoveEmoji(event) => {
                if let Some((portal_id, thread)) = self.lookup_portal(event.channel_id) {
                    if let Ok(Some(msg)) = self
                        .bridge
                        .db
                        .message_get_by_discord_id(portal_id, event.message_id)
                        .await
                    {
                        self.send_portal_event(
                            portal_id,
                            PortalEvent::ReactionDeleteKey(
                                msg.id,
                                bridge::ReactionKey::Discord(event.emoji),
                            ),
                            thread,
                        );
                    }
                }
            }
            GatewayEvent::ThreadCreate(event) => {
                self.route_thread_create((*event).0);
            }
            GatewayEvent::ThreadUpdate(event) => {
                let thread = (*event).0;
                self.route_portal_event(
                    thread.id,
                    PortalEvent::ChannelUpdate(ChannelData::Discord {
                        channel: Box::new(thread),
                        webhook: None,
                    }),
                );
            }
            GatewayEvent::ThreadDelete(event) => {
                self.thread_parents.remove(&event.id);
                if self.lookup_portal(event.id).is_some() {
                    self.route_portal_event(event.id, PortalEvent::ChannelDelete);
                }
            }
            GatewayEvent::ThreadListSync(event) => {
                for thread in event.threads {
                    if let Some(portal_id) = self.portal_lookup.get(&thread.id).copied() {
                        self.send_portal_event(portal_id, PortalEvent::Backfill, None);
                        continue;
                    }
                    if let Some(portal) = self.init_portal_from_db(thread.id).await {
                        self.send_portal_event(portal.id, PortalEvent::Backfill, None);
                        continue;
                    }
                    self.route_thread_create(thread);
                }
            }
            GatewayEvent::ThreadMembersUpdate(event) => {
                debug!(thread_id = %event.id, "ignoring thread members update");
            }
            GatewayEvent::GuildCreate(guild) => {
                if let GuildCreate::Available(guild) = *guild {
                    self.handle_guild_create(guild).await;
                }
            }
            _ => debug!("unhandled discord event"),
        }
    }

    /// queue a refetch of a partial message update
    ///
    /// fetching happens in a background task so a rate limited request can't
    /// stall the gateway event loop
    fn queue_message_update(&mut self, update: Message) {
        self.update_generation += 1;
        let generation = self.update_generation;
        let id = update.id;

        // coalesce updates that arrive while a fetch for the same message is
        // still in flight; only the latest state matters
        if let Some(pending) = self.pending_updates.get_mut(&id) {
            pending.generation = generation;
            pending.message = update;
            return;
        }

        let channel_id = update.channel_id;
        self.pending_updates.insert(
            id,
            PendingUpdate {
                generation,
                message: update,
            },
        );
        self.spawn_message_fetch(channel_id, id, generation);
    }

    fn spawn_message_fetch(&self, channel_id: ChannelId, id: MessageId, generation: u64) {
        let http = self.http.clone();
        let tx = self.updates_tx.clone();

        tokio::spawn(async move {
            let fetched = match http.message(channel_id, id).await {
                Ok(response) => match response.model().await {
                    Ok(message) => Some(message),
                    Err(e) => {
                        warn!(message_id = %id, "failed to fetch updated message: {e:?}");
                        None
                    }
                },
                Err(e) => {
                    warn!(message_id = %id, "failed to fetch updated message: {e:?}");
                    None
                }
            };

            let _ = tx.send((id, generation, fetched));
        });
    }

    fn handle_fetched_update(&mut self, id: MessageId, generation: u64, fetched: Option<Message>) {
        let Some(pending) = self.pending_updates.get(&id) else {
            return;
        };

        // a newer update arrived while this one was being fetched
        if pending.generation != generation {
            let channel_id = pending.message.channel_id;
            let generation = pending.generation;
            self.spawn_message_fetch(channel_id, id, generation);
            return;
        }

        let pending = self.pending_updates.remove(&id).expect("checked above");
        let message = fetched.unwrap_or(pending.message);
        self.route_portal_event(
            message.channel_id,
            PortalEvent::MessageUpdate(MessageData::Discord {
                message: Box::new(message),
            }),
        );
    }

    async fn handle_guild_create(&mut self, guild: Guild) {
        let realm_id = self.realm_data.iter().find_map(|(id, realm)| {
            if let Some(r_discord) = &realm.discord {
                if r_discord.guild_id == guild.id && realm.continuous {
                    return Some(*id);
                }
            }
            None
        });

        for channel in guild.channels.iter().chain(guild.threads.iter()) {
            // catch up on anything that was bridged while we were disconnected
            if let Some(portal_id) = self.portal_lookup.get(&channel.id).copied() {
                self.send_portal_event(portal_id, PortalEvent::Backfill, None);
                continue;
            }

            // the portal may exist even if its init event hasn't been handled
            // yet, so consult the database before treating the channel as new
            if let Some(portal) = self.init_portal_from_db(channel.id).await {
                self.send_portal_event(portal.id, PortalEvent::Backfill, None);
                continue;
            }

            match channel.kind {
                ChannelType::PublicThread
                | ChannelType::PrivateThread
                | ChannelType::AnnouncementThread => {
                    self.route_thread_create(channel.clone());
                }
                ChannelType::GuildText | ChannelType::GuildAnnouncement => {
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
        http: &Arc<HttpClient>,
        channels_creating: &Arc<Mutex<HashSet<ChannelId>>>,
        channel: Channel,
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

        let is_text = matches!(
            channel.kind,
            ChannelType::GuildText | ChannelType::GuildAnnouncement
        );
        let channel_data = if is_text {
            let webhook = match http.create_webhook(channel_id, "bridge").await {
                Ok(r) => match r.model().await {
                    Ok(wh) => wh,
                    Err(e) => {
                        error!(?e, "failed to create webhook");
                        channels_creating.lock().unwrap().remove(&channel_id);
                        return;
                    }
                },
                Err(e) => {
                    error!(?e, "failed to create webhook");
                    channels_creating.lock().unwrap().remove(&channel_id);
                    return;
                }
            };

            let Some(webhook_token) = webhook.token.as_deref() else {
                error!("webhook response is missing its token");
                channels_creating.lock().unwrap().remove(&channel_id);
                return;
            };

            let webhook_url = webhook_url(webhook.id, webhook_token);

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

    fn route_thread_create(&mut self, thread: Channel) {
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
                .channel(channel_id)
                .and_then(|channel| channel.parent_id)
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
                let msg = msg.to_owned();
                tokio::spawn(async move {
                    let _ = http.create_message(channel_id).content(&msg).await;
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
                let msg = msg.to_owned();
                tokio::spawn(async move {
                    let _ = http.create_message(channel_id).content(&msg).await;
                });
            }
            _ => {} // other events not relevant to Discord platform
        }
    }

    /// initialize a portal that exists in the database but isn't loaded yet
    ///
    /// used for guilds/threads discovered by the gateway before the persisted
    /// portals are initialized, so they are not bridged again.
    async fn init_portal_from_db(&mut self, channel_id: ChannelId) -> Option<Portal> {
        let Some(portal) = self
            .bridge
            .db
            .portal_get_by_discord_channel(channel_id.to_string())
            .await
            .inspect_err(|err| error!(%channel_id, "failed to look up portal: {err}"))
            .ok()
            .flatten()
        else {
            return None;
        };

        if !self.portal_data.contains_key(&portal.id) {
            let handle = self.bridge.create_portal_handle(portal.id);
            self.init_portal(&portal, &handle);
        }

        Some(portal)
    }

    fn init_portal(&mut self, portal: &Portal, handle: &PortalHandle) {
        // the portal may have been initialized already (eg. loaded from the
        // database before its `PortalInit` event was handled). keep the handle
        // its task is subscribed to instead of spawning a second task.
        if self.portal_data.contains_key(&portal.id) {
            return;
        }

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

async fn run_gateway(token: String, tx: mpsc::Sender<GatewayEvent>) {
    let mut shard = Shard::new(ShardId::ONE, token.clone(), Intents::all());
    let mut reconnect_delay = RECONNECT_MIN_DELAY;

    loop {
        while let Some(item) = shard.next_event(EventTypeFlags::all()).await {
            reconnect_delay = RECONNECT_MIN_DELAY;
            match item {
                Ok(event) => {
                    if tx.send(event).await.is_err() {
                        return;
                    }
                }
                Err(e) => warn!(?e, "discord gateway error"),
            }
        }

        let state = shard.state();
        let session = shard.session().cloned();
        let resume_url = shard.resume_url().map(str::to_owned);
        error!(?state, "discord gateway disconnected, reconnecting");

        tokio::time::sleep(reconnect_delay).await;
        reconnect_delay = (reconnect_delay * 2).min(RECONNECT_MAX_DELAY);

        let mut config = ConfigBuilder::new(token.clone(), Intents::all());
        if let (Some(session), Some(resume_url)) = (session, resume_url) {
            config = config.session(session).resume_url(resume_url);
        }
        shard = Shard::with_config(ShardId::ONE, config.build());
    }
}

fn webhook_url(id: WebhookId, token: &str) -> url::Url {
    format!("{DISCORD_API_BASE}/webhooks/{id}/{token}")
        .parse()
        .expect("webhook id and token always form a valid url")
}

async fn respond(http: &HttpClient, interaction: &Interaction, content: &str) -> Result<()> {
    let response = InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            content: Some(content.to_owned()),
            flags: Some(MessageFlags::EPHEMERAL),
            ..Default::default()
        }),
    };

    http.interaction(interaction.application_id)
        .create_response(interaction.id, &interaction.token, &response)
        .await?;

    Ok(())
}
