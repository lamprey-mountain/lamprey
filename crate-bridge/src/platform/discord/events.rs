use async_trait::async_trait;
use serenity::all::{
    ChannelId, Context, EventHandler, Guild, GuildChannel, GuildId, GuildMemberUpdateEvent,
    Interaction, Message, MessageId, MessageUpdateEvent, Presence, Reaction, Ready,
    TypingStartEvent,
};
use tokio::sync::mpsc;
use tracing::{error, info, trace};

use crate::{
    config::Config,
    platform::discord::interactions::{SlashCommand, get_commands, parse_interaction},
};

pub struct Handler {
    pub tx: mpsc::Sender<DiscordEvent>,
    pub config: Config,
}

pub enum DiscordEvent {
    MessageCreate(Message),
    MessageUpdate(MessageUpdateEvent, Option<Message>),
    MessageDelete(ChannelId, MessageId),
    ReactionAdd(Reaction),
    ReactionRemove(Reaction),
    ReactionRemoveAll(ChannelId, MessageId),
    ReactionRemoveEmoji(Reaction),
    ChannelCreate(GuildChannel),
    ChannelDelete(GuildChannel),
    InteractionCreate(SlashCommand),
    TypingStart(TypingStartEvent),
    PresenceUpdate(Presence),
    GuildMemberUpdate(GuildMemberUpdateEvent),
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("discord ready {}", ready.user.name);

        if let Err(err) = ctx
            .http
            .create_global_commands(&get_commands(&self.config))
            .await
        {
            error!("error while registering commands: {err:?}")
        }
    }

    async fn guild_create(&self, _ctx: Context, guild: Guild, _is_new: Option<bool>) {
        info!("discord guild create: {}", guild.name);
    }

    async fn message(&self, _ctx: Context, message: Message) {
        info!("discord message create: {:?}", message.content);
        let _ = self.tx.send(DiscordEvent::MessageCreate(message)).await;
    }

    async fn message_update(
        &self,
        _ctx: Context,
        _old: Option<Message>,
        new: Option<Message>,
        event: MessageUpdateEvent,
    ) {
        info!("discord message update: {:?}", event.id);
        let _ = self.tx.send(DiscordEvent::MessageUpdate(event, new)).await;
    }

    async fn message_delete(
        &self,
        _ctx: Context,
        channel_id: ChannelId,
        deleted_message_id: MessageId,
        _guild_id: Option<GuildId>,
    ) {
        info!("discord message delete: {:?}", deleted_message_id);
        let _ = self
            .tx
            .send(DiscordEvent::MessageDelete(channel_id, deleted_message_id))
            .await;
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
        let _ = self.tx.send(DiscordEvent::ReactionAdd(add_reaction)).await;
    }

    async fn reaction_remove(&self, _ctx: Context, removed_reaction: Reaction) {
        info!("discord reaction remove: {:?}", removed_reaction.emoji);
        let _ = self
            .tx
            .send(DiscordEvent::ReactionRemove(removed_reaction))
            .await;
    }

    async fn reaction_remove_all(
        &self,
        _ctx: Context,
        channel_id: ChannelId,
        removed_from_message_id: MessageId,
    ) {
        info!("discord reaction remove all: {:?}", removed_from_message_id);
        let _ = self
            .tx
            .send(DiscordEvent::ReactionRemoveAll(
                channel_id,
                removed_from_message_id,
            ))
            .await;
    }

    async fn reaction_remove_emoji(&self, _ctx: Context, removed_reactions: Reaction) {
        info!(
            "discord reaction remove emoji: {:?}",
            removed_reactions.emoji
        );
        let _ = self
            .tx
            .send(DiscordEvent::ReactionRemoveEmoji(removed_reactions))
            .await;
    }

    async fn typing_start(&self, _ctx: Context, event: TypingStartEvent) {
        info!("discord typing start: {:?}", event.user_id);
        let _ = self.tx.send(DiscordEvent::TypingStart(event)).await;
    }

    async fn channel_create(&self, _ctx: Context, channel: GuildChannel) {
        info!("discord channel create: {:?}", channel.name);
        let _ = self.tx.send(DiscordEvent::ChannelCreate(channel)).await;
    }

    async fn channel_delete(
        &self,
        _ctx: Context,
        channel: GuildChannel,
        _messages: Option<Vec<Message>>,
    ) {
        info!("discord channel delete: {:?}", channel.name);
        let _ = self.tx.send(DiscordEvent::ChannelDelete(channel)).await;
    }

    async fn interaction_create(&self, _ctx: Context, interaction: Interaction) {
        info!("interaction create");
        if let Some(command) = interaction.command() {
            match parse_interaction(command) {
                Ok(parsed) => {
                    let _ = self.tx.send(DiscordEvent::InteractionCreate(parsed)).await;
                }
                Err(e) => {
                    error!("error parsing interaction: {e:?}");
                }
            }
        }
    }

    async fn guild_member_update(
        &self,
        _ctx: Context,
        _old: Option<serenity::model::guild::Member>,
        _new: Option<serenity::model::guild::Member>,
        event: GuildMemberUpdateEvent,
    ) {
        info!("discord guild member update: {:?}", event.user.name);
        let _ = self.tx.send(DiscordEvent::GuildMemberUpdate(event)).await;
    }

    async fn presence_update(&self, _ctx: Context, presence: Presence) {
        trace!("discord presence update for user {}", presence.user.id);
        let _ = self.tx.send(DiscordEvent::PresenceUpdate(presence)).await;
    }

    // TODO: handle user_update
}
