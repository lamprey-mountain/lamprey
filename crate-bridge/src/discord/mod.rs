use crate::{bridge::BridgeHandle, config::DiscordConfig};
use serenity::all::GatewayIntents;

mod events;
mod interactions;

// re export discord (serenity) types
pub use serenity::all::{
    Attachment, AttachmentId, ChannelId, CreateEmbed, Embed, GuildId, Message, MessageId, User,
    UserId,
};

// TODO: listen to bridge.events
pub fn spawn(bridge: BridgeHandle, config: DiscordConfig) {
    let bridge = bridge.clone();
    tokio::spawn(async move {
        let handler = events::Handler { bridge };
        let mut client = serenity::Client::builder(
            &config.token.load().expect("failed to load token"),
            GatewayIntents::all(),
        )
        .event_handler(handler)
        .await
        .expect("Error creating client");

        if let Err(why) = client.start().await {
            eprintln!("Client error: {:?}", why);
        }
    });
}
