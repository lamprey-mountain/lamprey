use crate::v1::types::{AutomodRuleId, ChannelId, MessageId, RoleId, RoomId, UserId};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct AutomodRule {
    pub id: AutomodRuleId,
    pub room_id: RoomId,
    #[schema(max_length = 64)]
    pub name: String,
    pub enabled: bool,
    // TODO: support multiple triggers
    pub trigger: AutomodTrigger,
    #[schema(max_items = 8)]
    pub actions: Vec<AutomodAction>,
    pub except_roles: Vec<RoleId>,
    pub except_channels: Vec<ChannelId>,
    // /// whether this rule should affect everyone. actions aren't necessarily executed (eg. admins wont be timed out)
    // pub include_everyone: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct AutomodRuleCreate {
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: String,
    pub trigger: AutomodTrigger,
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8)))]
    pub actions: Vec<AutomodAction>,
    #[serde(default)]
    pub except_roles: Vec<RoleId>,
    #[serde(default)]
    pub except_channels: Vec<ChannelId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct AutomodRuleUpdate {
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub trigger: Option<AutomodTrigger>,
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8)))]
    pub actions: Option<Vec<AutomodAction>>,
    pub except_roles: Option<Vec<RoleId>>,
    pub except_channels: Option<Vec<ChannelId>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct AutomodRuleExecution {
    /// the rule that was executed
    pub rule: AutomodRule,

    /// the user who triggered this rule
    pub user_id: UserId,

    /// the channel this happened in (for messages)
    pub channel_id: Option<ChannelId>,

    /// the message this matched (excluded for Block)
    pub message_id: Option<MessageId>,

    /// the content that was matched against (eg. message content)
    pub content: String,

    /// the keyword or regex that was matched in the content
    pub matched: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum AutomodTrigger {
    /// scan a message based on regex
    MessageRegex {
        // max length 32
        deny: Vec<String>,

        // max length 32
        allow: Vec<String>,
    },

    /// scan a message based on its keywords. automatically adds word boundaries and decancers the string (ie. properly handles unicode lookalikes).
    MessageKeywords {
        // max length 32
        keywords: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum AutomodAction {
    /// block the message from being sent
    Block,

    /// timeout a user
    Timeout {
        /// in milliseconds
        duration: u64,
    },

    /// remove a message. unlike Block, remove messages can be allowed/restored by a moderator.
    Remove,

    /// send an alert to a channel
    SendAlert {
        /// where to send the alert to
        channel_id: ChannelId,
    },
    // TODO: automatic reactions?
    // /// add a reaction to the message
    // React {
    //     /// the reaction to add
    //     // TODO: use ReactionKeyParam here? or at least for patching
    //     reaction: ReactionKey,
    // },
}
