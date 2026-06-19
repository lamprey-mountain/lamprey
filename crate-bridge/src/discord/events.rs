use async_trait::async_trait;
use serenity::all::{Context, EventHandler, Guild, Message, MessageUpdateEvent, Ready};
use tracing::{error, info};

use crate::{bridge::BridgeHandle, discord::interactions::get_commands};

pub(super) struct Handler {
    pub bridge: BridgeHandle,
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
        // TODO: Map to BridgeEvent/PortalEvent
    }

    async fn message_update(
        &self,
        _ctx: Context,
        _old: Option<Message>,
        new: Option<Message>,
        _event: MessageUpdateEvent,
    ) {
        // drop update if new is None
        todo!()
    }

    // TODO: handle more events
    // crate-bridge-old/src/discord/events.rs
}
