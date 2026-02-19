use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serenity::all::{
    ChannelType, Guild, GuildChannel, GuildMemberUpdateEvent, Interaction, Message,
    MessagePagination, MessageUpdateEvent, Presence, Ready,
};
use serenity::model::prelude::{ChannelId, GuildId, MessageId, Reaction, TypingStartEvent};
use serenity::prelude::*;
use tracing::{debug, error, info, Instrument};

use crate::bridge::BridgeMessage;
use crate::bridge_common::{Globals, GlobalsTrait};
use crate::db::Data;
use crate::discord::commands::{get_commands, handle_interaction};
use crate::discord::presence::process_presence_update;
use crate::portal::{Portal, PortalMessage};

pub(super) struct GlobalsKey;

pub(super) struct Handler;

impl TypeMapKey for GlobalsKey {
    type Value = Arc<Globals>;
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("discord ready {}", ready.user.name);

        if let Err(err) = ctx.http.create_global_commands(&get_commands()).await {
            error!("error while registering commands: {err:?}")
        }
    }

    async fn guild_create(&self, ctx: Context, guild: Guild, _is_new: Option<bool>) {
        info!("discord guild create");
        let ctx_data = ctx.data.read().await;
        let globals = ctx_data.get::<GlobalsKey>().unwrap();

        async {
            let chans = guild.channels.values().chain(&guild.threads);
            for ch in chans {
                async {
                    if globals
                        .get_portal_by_discord_channel(ch.id)
                        .await
                        .unwrap()
                        .is_some()
                    {
                        let config = globals
                            .get_portal_by_discord_channel(ch.id)
                            .await
                            .unwrap()
                            .unwrap();
                        let portal = globals
                            .portals
                            .entry(config.lamprey_thread_id)
                            .or_insert_with(|| Portal::summon(globals.clone(), config.to_owned()));

                        let last_id = globals.last_discord_ids.get(&ch.id).map(|v| *v.value());
                        let Some(last_id) = last_id else {
                            return;
                        };
                        let mut p = MessagePagination::After(last_id);
                        loop {
                            let msgs = ctx
                                .http
                                .get_messages(ch.id, Some(p), Some(100))
                                .await
                                .unwrap();
                            if msgs.is_empty() {
                                break;
                            }
                            info!("discord backfill {} messages", msgs.len());
                            let last_id = msgs.first().unwrap().id;
                            for message in msgs.into_iter().rev() {
                                if globals.get_message_dc(message.id).await.unwrap().is_some() {
                                    debug!("skipping already bridged message: {}", message.id);
                                    continue;
                                }
                                let _ =
                                    portal.send(PortalMessage::DiscordMessageCreate { message });
                            }
                            p = MessagePagination::After(last_id);
                        }
                    } else {
                        if ch.kind != ChannelType::Text && ch.kind != ChannelType::News {
                            return;
                        }

                        let Ok(realms) = globals.get_realms().await else {
                            return;
                        };

                        let Some(realm_config) =
                            realms.iter().find(|c| c.discord_guild_id == guild.id)
                        else {
                            return;
                        };

                        if !realm_config.continuous {
                            return;
                        }

                        info!("no portal exists so we'll create one");

                        if let Err(e) =
                            globals
                                .bridge_chan
                                .send(BridgeMessage::DiscordChannelCreate {
                                    guild_id: guild.id,
                                    channel_id: ch.id,
                                    channel_name: ch.name.clone(),
                                    channel_type: ch.kind,
                                    parent_id: ch.parent_id,
                                })
                        {
                            error!("failed to send discord channel create message: {e}");
                        }
                    }
                }
                .instrument(tracing::debug_span!("incremental backfill channel", ?ch.id))
                .await;
            }
        }
        .instrument(tracing::debug_span!("incremental backfill guild", ?guild.id))
        .await;
    }

    async fn message(&self, ctx: Context, message: Message) {
        info!("discord message create");
        let ctx_data = ctx.data.read().await;
        let globals = ctx_data.get::<GlobalsKey>().unwrap();

        // ignore bridged messages
        if let Some(w) = message.webhook_id {
            if let Ok(Some(h)) = globals
                .get_portal_by_discord_channel(message.channel_id)
                .await
            {
                if let Ok(webhook) = Webhook::from_url(&ctx.http, &h.discord_webhook).await {
                    if webhook.id == w {
                        return;
                    }
                }
            }
        }

        let mut message_with_full_author = message.clone();
        let user_id = message.author.id;

        if message.webhook_id.is_none() {
            let cached_user = globals.discord_user_cache.get(&user_id);
            if cached_user.is_some()
                && cached_user.as_ref().unwrap().fetched_at.elapsed().as_secs() < 300
            {
                message_with_full_author.author = cached_user.unwrap().user.clone();
            } else {
                match ctx.http.get_user(user_id).await {
                    Ok(user) => {
                        globals.discord_user_cache.insert(
                            user_id,
                            crate::bridge_common::UserCacheEntry {
                                user: user.clone(),
                                fetched_at: std::time::Instant::now(),
                            },
                        );
                        message_with_full_author.author = user;
                    }
                    Err(e) => error!("Failed to fetch full user object for {}: {}", user_id, e),
                }
            }
        }

        globals
            .portal_send_dc(
                message
                    .thread
                    .as_ref()
                    .map(|t| t.id)
                    .unwrap_or(message.channel_id),
                PortalMessage::DiscordMessageCreate {
                    message: message_with_full_author,
                },
            )
            .await;
    }

    async fn message_update(
        &self,
        ctx: Context,
        _old_if_available: Option<Message>,
        new: Option<Message>,
        event: MessageUpdateEvent,
    ) {
        info!("discord message update");
        let ctx_data = ctx.data.read().await;
        let globals = ctx_data.get::<GlobalsKey>().unwrap();

        // ignore bridged messages
        if let Some(w) = new.as_ref().and_then(|m| m.webhook_id) {
            if let Ok(Some(h)) = globals
                .get_portal_by_discord_channel(event.channel_id)
                .await
            {
                if let Ok(webhook) = Webhook::from_url(&ctx.http, &h.discord_webhook).await {
                    if webhook.id == w {
                        return;
                    }
                }
            }
        }

        globals
            .portal_send_dc(
                event.channel_id,
                PortalMessage::DiscordMessageUpdate {
                    update: event,
                    new_message: new,
                },
            )
            .await;
    }

    async fn message_delete(
        &self,
        ctx: Context,
        channel_id: ChannelId,
        deleted_message_id: MessageId,
        _guild_id: Option<GuildId>,
    ) {
        info!("discord message delete");
        let ctx_data = ctx.data.read().await;
        let globals = ctx_data.get::<GlobalsKey>().unwrap();
        globals
            .portal_send_dc(
                channel_id,
                PortalMessage::DiscordMessageDelete {
                    message_id: deleted_message_id,
                },
            )
            .await;
    }

    async fn message_delete_bulk(
        &self,
        ctx: Context,
        channel_id: ChannelId,
        multiple_deleted_messages_ids: Vec<MessageId>,
        _guild_id: Option<GuildId>,
    ) {
        info!("discord message delete bulk");
        let ctx_data = ctx.data.read().await;
        let globals = ctx_data.get::<GlobalsKey>().unwrap();
        for message_id in multiple_deleted_messages_ids {
            globals
                .portal_send_dc(
                    channel_id,
                    PortalMessage::DiscordMessageDelete { message_id },
                )
                .await;
        }
    }

    async fn reaction_add(&self, ctx: Context, add_reaction: Reaction) {
        info!("discord reaction add");
        let ctx_data = ctx.data.read().await;
        let globals = ctx_data.get::<GlobalsKey>().unwrap();
        globals
            .portal_send_dc(
                add_reaction.channel_id,
                PortalMessage::DiscordReactionAdd { add_reaction },
            )
            .await;
    }

    async fn reaction_remove(&self, ctx: Context, removed_reaction: Reaction) {
        info!("discord reaction remove");
        let ctx_data = ctx.data.read().await;
        let globals = ctx_data.get::<GlobalsKey>().unwrap();
        globals
            .portal_send_dc(
                removed_reaction.channel_id,
                PortalMessage::DiscordReactionRemove { removed_reaction },
            )
            .await;
    }

    async fn typing_start(&self, ctx: Context, event: TypingStartEvent) {
        info!("discord typing start");
        let ctx_data = ctx.data.read().await;
        let globals = ctx_data.get::<GlobalsKey>().unwrap();
        globals
            .portal_send_dc(
                event.channel_id,
                PortalMessage::DiscordTyping {
                    user_id: event.user_id,
                },
            )
            .await;
    }

    async fn channel_create(&self, ctx: Context, channel: GuildChannel) {
        info!("discord channel create");
        let ctx_data = ctx.data.read().await;
        let globals = ctx_data.get::<GlobalsKey>().unwrap();

        if globals
            .recently_created_discord_channels
            .remove(&channel.id)
            .is_some()
        {
            info!("ignoring discord channel create from bridge");
            return;
        }

        if !matches!(
            channel.kind,
            ChannelType::Text | ChannelType::News | ChannelType::Category
        ) {
            return;
        }

        let guild_id = channel.guild_id;

        if globals
            .get_portal_by_discord_channel(channel.id)
            .await
            .is_ok_and(|p| p.is_some())
        {
            return;
        }

        if let Err(e) = globals
            .bridge_chan
            .send(BridgeMessage::DiscordChannelCreate {
                guild_id,
                channel_id: channel.id,
                channel_name: channel.name.clone(),
                channel_type: channel.kind,
                parent_id: channel.parent_id,
            })
        {
            error!("failed to send discord channel create message: {e}");
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        info!("interaction create {interaction:?}");

        let Some(command) = interaction.into_command() else {
            return;
        };

        let ctx_data = ctx.data.read().await;
        let globals = ctx_data.get::<GlobalsKey>().unwrap().clone();

        handle_interaction(&ctx, command, globals).await;
    }

    async fn guild_member_update(
        &self,
        ctx: Context,
        old_if_available: Option<serenity::model::guild::Member>,
        new: Option<serenity::model::guild::Member>,
        _event: GuildMemberUpdateEvent,
    ) {
        let Some(new) = new else {
            return;
        };
        info!("discord guild member update");
        let ctx_data = ctx.data.read().await;
        let globals = ctx_data.get::<GlobalsKey>().unwrap().clone();

        let old_nick = old_if_available.and_then(|m| m.nick);
        if old_nick == new.nick {
            return;
        }

        tokio::spawn(async move {
            let res: Result<()> = async {
                let Some(puppet) = globals
                    .get_puppet("discord", &new.user.id.to_string())
                    .await?
                else {
                    debug!("no puppet found for discord user {}", new.user.id);
                    return Ok(());
                };

                let realms = globals.get_realms().await?;
                let Some(realm) = realms.iter().find(|r| r.discord_guild_id == new.guild_id) else {
                    debug!("no realm found for guild {}", new.guild_id);
                    return Ok(());
                };

                let ly = globals.lamprey_handle().await?;

                let patch = common::v1::types::RoomMemberPatch {
                    override_name: Some(new.nick.clone()),
                    override_description: None,
                    mute: None,
                    deaf: None,
                    roles: None,
                    timeout_until: None,
                };

                ly.room_member_patch(realm.lamprey_room_id, puppet.id.into(), &patch)
                    .await?;
                info!("updated lamprey member nick for {}", new.user.id);
                Ok(())
            }
            .await;
            if let Err(e) = res {
                error!("failed to handle guild member update: {e}");
            }
        });
    }

    async fn presence_update(&self, ctx: Context, presence: Presence) {
        debug!("discord presence update for user {}", presence.user.id);
        let ctx_data = ctx.data.read().await;
        let globals = ctx_data.get::<GlobalsKey>().unwrap().clone();

        globals.presences.insert(presence.user.id, presence.clone());

        tokio::spawn(async move {
            if let Err(e) = process_presence_update(globals, presence).await {
                error!("failed to handle presence update: {e}");
            }
        });
    }
}

use serenity::all::Webhook;
