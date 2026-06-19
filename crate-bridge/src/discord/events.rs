use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serenity::all::{
    ChannelId, ChannelType, Context, EventHandler, Guild, GuildChannel, GuildId,
    GuildMemberUpdateEvent, Interaction, Message, MessageId, MessageUpdateEvent, Presence,
    Reaction, Ready, TypingStartEvent,
};
use tokio::sync::RwLock;
use tracing::{error, info, trace};

use crate::{
    bridge::{BridgeEvent, BridgeHandle, MessageData, PortalEvent, PortalHandle, PortalId},
    discord::interactions::get_commands,
};

pub(super) struct Handler {
    pub bridge: BridgeHandle,
    // PERF: use dashmap?
    pub portal_handles: Arc<RwLock<HashMap<PortalId, PortalHandle>>>,
    pub portal_lookup: Arc<RwLock<HashMap<ChannelId, PortalId>>>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("discord ready {}", ready.user.name);

        if let Err(err) = ctx.http.create_global_commands(&get_commands()).await {
            error!("error while registering commands: {err:?}")
        }
    }

    async fn guild_create(&self, _ctx: Context, guild: Guild, _is_new: Option<bool>) {
        info!("discord guild create: {}", guild.name);
    }

    async fn message(&self, _ctx: Context, message: Message) {
        info!("discord message create: {:?}", message.content);

        // TODO: helper method to look up a portal
        // TODO: helper method to send a PortalEvent to a channel_id
        let portal_lookup = self.portal_lookup.read().await;
        if let Some(portal_id) = portal_lookup.get(&message.channel_id) {
            let portal_handles = self.portal_handles.read().await;
            if let Some(handle) = portal_handles.get(portal_id) {
                let event = PortalEvent::MessageCreate(MessageData::Discord(message));
                let _ = handle.events.send(Arc::new(event)); // TODO: better error handling
            }
        }
    }

    async fn message_update(
        &self,
        _ctx: Context,
        _old: Option<Message>,
        _new: Option<Message>,
        _event: MessageUpdateEvent,
    ) {
        info!("discord message update: {:?}", _event.id);
        // TODO: Map to BridgeEvent/PortalEvent
    }

    async fn message_delete(
        &self,
        _ctx: Context,
        channel_id: ChannelId,
        deleted_message_id: MessageId,
        _guild_id: Option<GuildId>,
    ) {
        info!("discord message delete: {:?}", deleted_message_id);
        // TODO: Map to BridgeEvent/PortalEvent
    }

    async fn message_delete_bulk(
        &self,
        _ctx: Context,
        channel_id: ChannelId,
        multiple_deleted_messages_ids: Vec<MessageId>,
        _guild_id: Option<GuildId>,
    ) {
        info!(
            "discord message delete bulk: {:?}",
            multiple_deleted_messages_ids
        );
        // TODO: Map to BridgeEvent/PortalEvent
    }

    async fn reaction_add(&self, _ctx: Context, add_reaction: Reaction) {
        info!("discord reaction add: {:?}", add_reaction.emoji);
        // TODO: Map to BridgeEvent/PortalEvent
    }

    async fn reaction_remove(&self, _ctx: Context, removed_reaction: Reaction) {
        info!("discord reaction remove: {:?}", removed_reaction.emoji);
        // TODO: Map to BridgeEvent/PortalEvent
    }

    async fn typing_start(&self, _ctx: Context, event: TypingStartEvent) {
        info!("discord typing start: {:?}", event.user_id);
        // TODO: Map to BridgeEvent/PortalEvent
    }

    async fn channel_create(&self, _ctx: Context, channel: GuildChannel) {
        info!("discord channel create: {:?}", channel.name);
        // TODO: Map to BridgeEvent/PortalEvent
    }

    async fn interaction_create(&self, _ctx: Context, interaction: Interaction) {
        info!("interaction create");
        // TODO: Map to BridgeEvent/PortalEvent
    }

    async fn guild_member_update(
        &self,
        _ctx: Context,
        _old: Option<serenity::model::guild::Member>,
        _new: Option<serenity::model::guild::Member>,
        _event: GuildMemberUpdateEvent,
    ) {
        info!("discord guild member update");
        // TODO: Map to BridgeEvent/PortalEvent
    }

    async fn presence_update(&self, _ctx: Context, presence: Presence) {
        trace!("discord presence update for user {}", presence.user.id);
        // TODO: Map to BridgeEvent/PortalEvent
    }

    // TODO: handle user_update
}
