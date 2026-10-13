use lamprey_macros::record;

use crate::v1::types::{Embed, MessageCreate, MessagePatch, interactions::InteractionType};

/// respond to an interaction
#[record]
pub struct Create {
    #[serde(flatten)]
    pub ty: CreateType,
}

// TODO: more response types (autocomplete, open modal)
#[record]
#[serde(tag = "type")]
pub enum CreateType {
    /// for webhooks
    Pong,

    /// reply with a message
    Reply { message: MessageCreate },

    /// show a loading indicator, will reply later
    ReplyDefer,

    /// edit the message this button is attached to
    MessageUpdate { patch: MessagePatch },

    /// acknowledge an interaction, does not show a loading indicator
    Defer,

    /// unfurl a url
    Unfurl {
        /// also generate the default url preview
        include_default: bool,

        /// generated these embeds
        embeds: Vec<Embed>,
    },
}

/// an interaction has been responded to
#[record]
pub struct Response {
    // TODO: return extra data here (like what?)
}

impl CreateType {
    /// get whether this response can be created for an interaction type
    pub fn can_respond_to(&self, ty: &InteractionType) -> bool {
        match (self, ty) {
            (CreateType::Pong, InteractionType::Ping) => true,

            (CreateType::Reply { .. }, InteractionType::Button(_)) => true,
            (CreateType::ReplyDefer, InteractionType::Button(_)) => true,
            (CreateType::MessageUpdate { .. }, InteractionType::Button(_)) => true,
            (CreateType::Defer, InteractionType::Button(_)) => true,

            (CreateType::Defer, InteractionType::Unfurl(_)) => true,
            (CreateType::Unfurl { .. }, InteractionType::Unfurl(_)) => true,

            // maybe?
            // (CreateType::Reply { .. }, InteractionType::Unfurl(_)) => true,
            // (CreateType::ReplyDefer, InteractionType::Unfurl(_)) => true,
            // (CreateType::MessageUpdate { .. }, InteractionType::Unfurl(_)) => true,
            (_, _) => false,
        }
    }
}
