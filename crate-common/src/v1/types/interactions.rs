// for things ike buttons
// somewhat copied from discord since they do things reasonably

use crate::v1::types::{
    ApplicationId, Channel, ChannelId, Message, MessageCreate, MessageId, MessagePatch, Permission,
    Room, RoomMember, User, UserId,
};

use super::ids::InteractionId;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

/*
// POST /interaction
// POST /interaction/{interaction_id}/{token}/callback

struct Application {
    interactions_url: Option<Url>,
}

enum MessageSync {
    // sent to the user and application
    InteractionCreate {
        user_id: Option<UserId>,

        // only sent to the user
        // use Ideompotency-Key
        nonce: Option<String>,

        interaction: Interaction,
        application_id: ApplicationId,
    },

    InteractionSuccess {
        interaction_id: InteractionId,
    },

    InteractionFailure {
        interaction_id: InteractionId,
    },
}
*/

/// create a new interaction
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct InteractionCreate {
    pub application_id: ApplicationId,

    #[cfg_attr(feature = "serde", serde(flatten))]
    pub ty: InteractionCreateType,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum InteractionCreateType {
    /// a button was pressed
    Button {
        channel_id: ChannelId,
        message_id: MessageId,
        custom_id: String,
    },
}

/// an interaction was created
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Interaction {
    pub id: InteractionId,
    pub application_id: ApplicationId,

    /// unique token for responding to this interaction. this exists so you don't need to give your token to an http server for http based interactions
    pub token: String,

    /// always 1 currently
    pub version: u16,

    #[cfg_attr(feature = "serde", serde(flatten))]
    pub ty: InteractionType,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum InteractionType {
    /// for webhooks
    Ping,

    /// a button was pressed
    Button {
        /// the room this interaction was created in
        room: Option<Room>,

        /// the channel this interaction was created in
        channel: Channel,

        /// the message this button was attached to
        message: Message,

        /// the user who initiated this interaction
        user: User,

        /// the room member for the user who initiated this interaction
        room_member: Option<RoomMember>,

        /// the permissions the user has in the target channel
        user_permissions: Vec<Permission>,

        /// the permissions the application has in the target channel
        application_permissions: Vec<Permission>,

        /// application defined id associated with this button
        custom_id: String,
    },
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct InteractionResponse {
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub ty: InteractionResponseType,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum InteractionResponseType {
    /// for webhooks
    Pong,

    /// reply with a message
    Reply {
        #[cfg_attr(feature = "serde", serde(flatten))]
        message: MessageCreate,
    },

    /// show a loading indicator, will reply later
    ReplyDefer,

    /// edit the message this button is attached to
    MessageUpdate {
        #[cfg_attr(feature = "serde", serde(flatten))]
        patch: MessagePatch,
    },

    /// acknowledge an interaction, does not show a loading indicator
    Defer,
}

// TODO: move these to message.rs

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageComponent {
    pub custom_id: Option<String>,

    pub components: Vec<MessageComponent>,

    #[cfg_attr(feature = "serde", serde(flatten))]
    pub ty: MessageComponentType,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum MessageComponentType {
    Button { label: String },
}

impl MessageComponentType {
    pub fn has_children(&self) -> bool {
        false
    }

    pub fn requires_custom_id(&self) -> bool {
        matches!(self, MessageComponentType::Button { .. })
    }
}

/// the interaction that caused this message to be sent
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageInteraction {
    pub id: InteractionId,
    pub application_id: ApplicationId,

    /// the user who triggered this interaction
    pub user_id: UserId,

    /// the target message the button/component was on
    pub target_message_id: Option<MessageId>,
}
