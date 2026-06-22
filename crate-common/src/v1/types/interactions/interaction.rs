use crate::v1::types::{
    ApplicationId, Channel, ChannelId, Embed, InteractionId, Message, MessageCreate, MessageId,
    MessagePatch, Permission, Room, RoomMember, User, UserId,
};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

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

/// an user interacted with your application
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Interaction {
    pub id: InteractionId,
    pub application_id: ApplicationId,

    /// unique token for responding to this interaction
    ///
    /// this exists so you don't need to give your token to an http server for http based interactions. only is set for bots.
    pub token: Option<String>,

    /// always 1 currently
    pub version: u16,

    #[cfg_attr(feature = "serde", serde(flatten))]
    pub ty: InteractionType,
}

// TODO: refactor out common interaction context, Ping is only for webhooks anyways
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

    /// unfurl a url
    Unfurl {
        /// the room this interaction was created in
        room: Option<Room>,

        /// the channel this interaction was created in
        channel: Channel,

        /// the message this link is contained in
        message: Message,

        /// the user who send the message
        user: User,

        /// the room member for the user who initiated this interaction
        room_member: Option<RoomMember>,

        /// the permissions the user has in the target channel
        user_permissions: Vec<Permission>,

        /// the permissions the application has in the target channel
        application_permissions: Vec<Permission>,
    },
    // TODO: add locale?
    // TODO: add authorizers?
}

/// respond to an interaction
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct InteractionResponseCreate {
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub ty: InteractionResponseCreateType,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum InteractionResponseCreateType {
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
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct InteractionResponse {
    // TODO: return extra data here
}

impl InteractionType {
    /// whether messages can be sent in reply to this interaction
    // TODO: use this
    pub fn can_reply(&self) -> bool {
        matches!(self, InteractionType::Button { .. })
    }

    /// get the id of the channel this interaction exists in
    pub fn channel_id(&self) -> Option<ChannelId> {
        match self {
            Self::Ping => None,
            Self::Button { channel, .. } => Some(channel.id),
            Self::Unfurl { channel, .. } => Some(channel.id),
        }
    }

    /// get the id of the user who created this interaction
    pub fn user_id(&self) -> Option<UserId> {
        match self {
            Self::Ping => None,
            Self::Button { user, .. } => Some(user.id),
            Self::Unfurl { user, .. } => Some(user.id),
        }
    }

    /// get the id of the message that this interaction was created from
    fn source_message_id(&self) -> Option<MessageId> {
        match self {
            Self::Button { message, .. } => Some(message.id),
            Self::Ping => None,
            Self::Unfurl { message, .. } => Some(message.id),
        }
    }
}

impl InteractionCreateType {
    pub fn channel_id(&self) -> Option<ChannelId> {
        match self {
            InteractionCreateType::Button { channel_id, .. } => Some(*channel_id),
        }
    }
}
