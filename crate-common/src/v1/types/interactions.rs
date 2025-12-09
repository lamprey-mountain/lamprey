// for things ike buttons
// somewhat copied from discord since they do things reasonably

use crate::v1::types::{
    misc::Color, ApplicationId, Channel, ChannelId, Message, MessageCreate, MessageId,
    MessagePatch, Permission, Room, RoomMember, User, UserId,
};

use super::ids::InteractionId;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use url::Url;
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
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub ty: MessageComponentType,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum MessageComponentType {
    /// a clickable button
    Button {
        label: String,
        style: ButtonStyle,

        /// required for non link buttons
        custom_id: Option<String>,

        /// what to link to, iff style == `Link`
        url: Option<Url>,
    },

    /// a group of other components
    Container {
        components: Vec<MessageComponent>,
        color: Option<Color>,
    },

    /// markdown text
    Text { content: String },
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ButtonStyle {
    #[default]
    Primary,
    Secondary,
    Danger,

    /// a link to another page
    Link,
}

impl MessageComponentType {
    pub fn is_interactive(&self) -> bool {
        match self {
            MessageComponentType::Button { style, .. } => style != &ButtonStyle::Link,
            MessageComponentType::Container { components, .. } => {
                components.iter().any(|c| c.ty.is_interactive())
            }
            MessageComponentType::Text { .. } => false,
        }
    }

    pub fn validate(&self) -> Vec<String> {
        match self {
            MessageComponentType::Button {
                style,
                url,
                label,
                custom_id,
                ..
            } => {
                let err_custom_id_len = if let Some(custom_id) = &custom_id {
                    if custom_id.is_empty() {
                        vec!["custom_id cannot be empty".to_owned()]
                    } else if custom_id.len() > 256 {
                        vec!["custom_id can have up to 256 chars".to_owned()]
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                };

                let err_labels = if label.is_empty() {
                    vec!["label cannot be empty".to_owned()]
                } else if label.len() > 256 {
                    vec!["label can have up to 256 chars".to_owned()]
                } else {
                    vec![]
                };

                let err_links = match (url.is_some(), style == &ButtonStyle::Link) {
                    (true, false) => vec!["only link buttons can have a url".to_owned()],
                    (false, true) => vec!["link buttons must have a url".to_owned()],
                    (_, _) => vec![],
                };

                let err_custom_id = if custom_id.is_some() && style == &ButtonStyle::Link {
                    vec!["link button cannot have custom_id".to_owned()]
                } else {
                    vec![]
                };

                let mut errs = vec![];
                errs.extend(err_custom_id_len);
                errs.extend(err_labels);

                errs.extend(err_links);
                errs.extend(err_custom_id);
                errs
            }
            MessageComponentType::Container { components, .. } => {
                if components.is_empty() {
                    vec!["containers cannot be empty".to_owned()]
                } else if components.len() > 10 {
                    vec!["containers can have up to 10 components".to_owned()]
                } else {
                    let mut errs = vec![];
                    for c in components {
                        if matches!(c.ty, MessageComponentType::Container { .. }) {
                            errs.push("containers can only contain type: Button".to_owned())
                        }

                        errs.extend(c.ty.validate());
                    }
                    errs
                }
            }
            MessageComponentType::Text { content: text } => {
                if text.is_empty() {
                    vec!["text cannot be empty".to_owned()]
                } else if text.len() > 8192 {
                    vec!["text can have up to 8192 chars".to_owned()]
                } else {
                    vec![]
                }
            }
        }
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

// struct Message {
//     components: Vec<MessageComponent>,
//     interaction: Option<MessageInteraction>,
// }
