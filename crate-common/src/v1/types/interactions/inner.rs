use lamprey_macros::record;
use url::Url;

use crate::v1::types::{
    ApplicationId, Channel, ChannelId, InteractionId, Message, MessageId, Permission, Room,
    RoomMember, User, UserId,
};

/// an user interacted with your application
#[record]
pub struct Interaction {
    pub id: InteractionId,
    pub application_id: ApplicationId,

    /// unique token for responding to this interaction
    ///
    /// this exists so you don't need to give your token to an http server for
    /// http based interactions. only is set for bots.
    pub token: Option<String>, // TODO: custom type for tokens, these should be secret

    /// always 1 currently
    pub version: u16, // TODO: InteractionVersion type

    #[serde(flatten)]
    pub ty: InteractionType,
}

// TODO: more interaction types (form, autocomplete, slash command(?), modal, application installed, select)
#[record]
#[serde(tag = "type")]
pub enum InteractionType {
    /// for webhooks
    Ping,

    Button(Button),
    Unfurl(Unfurl),
}

/// context for an interaction which occured in a channel
// TODO: add locale, authorizers, attachment_size_limit, ...?
// NOTE: maybe leave some fields empty for some unfurls? if i let websites define their own unfurling logic, to avoid leaking who send what link to websites.
// TODO: resolve some data, like users, members, roles, channels, messages, attachments, etc
#[record]
pub struct Context {
    /// the room this interaction was created in
    pub room: Option<Room>,

    /// the channel this interaction was created in
    pub channel: Channel,

    /// the message this interaction was created for
    ///
    /// eg. the message a button is on or the message that contains the link to unfurl
    pub message: Message,

    /// the user who initiated this interaction
    pub user: User,

    /// the room member for the user who initiated this interaction
    ///
    /// only exists if this interaction was created in a room
    pub room_member: Option<RoomMember>,

    /// the permissions the user has in the target channel
    // TODO: use a more compact representation?
    pub user_permissions: Vec<Permission>,

    /// the permissions the application has in the target channel
    pub application_permissions: Vec<Permission>,
}

/// a button was pressed
#[record]
pub struct Button {
    #[serde(flatten)]
    pub context: Context,

    /// application defined id associated with this button
    pub custom_id: String,
}

/// unfurl a url
#[record]
pub struct Unfurl {
    #[serde(flatten)]
    pub context: Context,

    /// the links to unfurl
    pub urls: Vec<Url>,
}

impl InteractionType {
    /// whether messages can be sent in reply to this interaction
    // TODO: use this
    pub fn can_reply(&self) -> bool {
        matches!(self, InteractionType::Button(_))
    }

    /// get the id of the channel this interaction exists in
    pub fn channel_id(&self) -> Option<ChannelId> {
        match self {
            Self::Ping => None,
            Self::Button(button) => Some(button.context.channel.id),
            Self::Unfurl(unfurl) => Some(unfurl.context.channel.id),
        }
    }

    /// get the id of the user who created this interaction
    pub fn user_id(&self) -> Option<UserId> {
        match self {
            Self::Ping => None,
            Self::Button(button) => Some(button.context.user.id),
            Self::Unfurl(unfurl) => Some(unfurl.context.user.id),
        }
    }

    /// get the id of the message that this interaction was created from
    pub fn source_message_id(&self) -> Option<MessageId> {
        match self {
            Self::Ping => None,
            Self::Button(button) => Some(button.context.message.id),
            Self::Unfurl(unfurl) => Some(unfurl.context.message.id),
        }
    }
}
