/// portal actor message
#[derive(Debug)]
pub enum PortalMessage {
    LampreyMessageCreate {
        message: common::v2::types::message::Message,
    },
    LampreyMessageUpdate {
        message: common::v2::types::message::Message,
    },
    LampreyMessageDelete {
        message_id: common::v1::types::MessageId,
    },
    DiscordMessageCreate {
        message: serenity::all::Message,
    },
    DiscordMessageUpdate {
        update: serenity::all::MessageUpdateEvent,
        new_message: Option<serenity::all::Message>,
    },
    DiscordMessageDelete {
        message_id: serenity::all::MessageId,
    },
    DiscordReactionAdd {
        add_reaction: serenity::all::Reaction,
    },
    DiscordReactionRemove {
        removed_reaction: serenity::all::Reaction,
    },
    DiscordTyping {
        user_id: serenity::model::id::UserId,
    },
}
