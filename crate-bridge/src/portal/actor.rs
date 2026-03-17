use std::fmt::Debug;
use std::sync::Arc;

use crate::bridge_common::{Globals, PortalConfig};

use anyhow::Result;
use kameo::message::{Context, Message};
use kameo::prelude::{ActorStopReason, WeakActorRef};
use serenity::all::ChannelId as DcChannelId;
use tracing::error;

use crate::portal::messages::PortalMessage;

pub struct Portal {
    pub globals: Arc<Globals>,
    pub config: PortalConfig,
}

impl kameo::Actor for Portal {
    type Args = (Arc<Globals>, PortalConfig);
    type Error = anyhow::Error;

    async fn on_start(
        args: Self::Args,
        _actor_ref: kameo::prelude::ActorRef<Self>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            globals: args.0,
            config: args.1,
        })
    }

    async fn on_panic(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        err: kameo::error::PanicError,
    ) -> Result<std::ops::ControlFlow<ActorStopReason>, Self::Error> {
        tracing::error!("Portal Actor panicked! Error: {:?}", err);
        Ok(std::ops::ControlFlow::Break(ActorStopReason::Panicked(err)))
    }

    async fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        reason: ActorStopReason,
    ) -> Result<(), Self::Error> {
        tracing::warn!("Portal Actor stopped. Reason: {:?}", reason);
        Ok(())
    }
}

impl Debug for Portal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Portal ({:?})", self.config)
    }
}

impl Portal {
    pub fn channel_id(&self) -> DcChannelId {
        self.config.discord_channel_id
    }

    pub fn thread_id(&self) -> common::v1::types::ChannelId {
        self.config.lamprey_thread_id
    }

    pub fn room_id(&self) -> common::v1::types::RoomId {
        self.config.lamprey_room_id
    }
}

impl Message<PortalMessage> for Portal {
    type Reply = Result<()>;

    async fn handle(
        &mut self,
        msg: PortalMessage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if let Err(e) = self.handle_inner(msg).await {
            error!("portal actor handler failed: {:?}", e);
            return Err(e);
        }
        Ok(())
    }
}

impl Portal {
    #[tracing::instrument(
        skip(self),
        fields(
            lamprey_thread_id = %self.config.lamprey_thread_id,
            discord_channel_id = %self.config.discord_channel_id,
        )
    )]
    async fn handle_inner(&mut self, msg: PortalMessage) -> Result<()> {
        match msg {
            PortalMessage::LampreyMessageCreate { message } => {
                self.handle_lamprey_message_create(message).await?;
            }
            PortalMessage::LampreyMessageUpdate { message } => {
                self.handle_lamprey_message_create(message).await?;
            }
            PortalMessage::LampreyMessageDelete { message_id } => {
                self.handle_lamprey_message_delete(message_id).await?;
            }
            PortalMessage::DiscordMessageCreate { message } => {
                self.handle_discord_message_create(message).await?;
            }
            PortalMessage::DiscordMessageUpdate {
                update,
                new_message,
            } => {
                self.handle_discord_message_update(update, new_message)
                    .await?;
            }
            PortalMessage::DiscordMessageDelete { message_id } => {
                self.handle_discord_message_delete(message_id).await?;
            }
            PortalMessage::DiscordReactionAdd { add_reaction } => {
                self.handle_discord_reaction_add(add_reaction).await?;
            }
            PortalMessage::DiscordReactionRemove { removed_reaction } => {
                self.handle_discord_reaction_remove(removed_reaction)
                    .await?;
            }
            PortalMessage::DiscordTyping { user_id } => {
                self.handle_discord_typing(user_id).await?;
            }
        }
        Ok(())
    }
}
