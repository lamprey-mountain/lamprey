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

    // TODO: support multiple triggers?
    pub trigger: AutomodTrigger,
    // /// execute this rule when ANY of these triggers match
    // #[cfg_attr(feature = "utoipa", schema(max_items = 8))]
    // pub triggers: Vec<AutomodTrigger>,
    /// when executed, do ALL of these actions
    #[cfg_attr(feature = "utoipa", schema(max_items = 8))]
    pub actions: Vec<AutomodAction>,

    /// what roles should be exempt from this rule. users with RoomManage are always exempt.
    pub except_roles: Vec<RoleId>,

    /// what channels should be exempt from this rule.
    pub except_channels: Vec<ChannelId>,
    /// if nsfw channels should be exempt from this rule.
    pub except_nsfw: bool,

    /// whether this rule should affect everyone. actions aren't necessarily executed (eg. admins wont be timed out)
    pub include_everyone: bool,

    pub target: AutomodTarget,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct AutomodRuleCreate {
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: String,
    pub trigger: AutomodTrigger,
    // #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8)))]
    // pub triggers: Vec<AutomodTrigger>,
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8)))]
    pub actions: Vec<AutomodAction>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub except_roles: Vec<RoleId>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub except_channels: Vec<ChannelId>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub except_nsfw: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub include_everyone: bool,
    pub target: AutomodTarget,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct AutomodRuleUpdate {
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub trigger: Option<AutomodTrigger>,
    // #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8)))]
    // pub triggers: Option<Vec<AutomodTrigger>>,
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8)))]
    pub actions: Option<Vec<AutomodAction>>,
    pub except_roles: Option<Vec<RoleId>>,
    pub except_channels: Option<Vec<ChannelId>>,
    pub except_nsfw: Option<bool>,
    pub include_everyone: Option<bool>,
    pub target: Option<AutomodTarget>,
}

/// minimal version of AutomodRule to prevent leaking the rule trigger
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct AutomodRuleStripped {
    pub id: AutomodRuleId,
    pub name: String,
    pub target: AutomodTarget,
}

/// what this rule should be evaluated on
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum AutomodTarget {
    /// messages, threads, voice statuses
    Content,

    /// user names, bios, and nicknames
    Member,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct AutomodRuleExecution {
    /// the rule that was executed
    pub rule: AutomodRule,

    /// the user who triggered this rule
    pub user_id: UserId,

    /// the channel this happened in (for messages)
    pub channel_id: Option<ChannelId>,

    /// the message this matched (excluded for Block)
    pub message_id: Option<MessageId>,

    /// the text that was matched against (eg. message content)
    pub text: Option<String>,

    /// the keyword or regex that was matched in the content
    pub text_matched: Option<AutomodMatches>,

    /// where this piece of text was found
    pub text_location: Option<AutomodTextLocation>,
}

/// request body for an automod test request
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct AutomodRuleTestRequest {
    /// the text to attempt to scan
    pub text: String,
}

/// response body for an automod test request
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct AutomodRuleTest {
    /// the rules that matched the text
    pub rules: Vec<AutomodRule>,

    /// the content that was matched
    pub matches: Option<AutomodMatches>,

    /// deduplicated list of all of the actions that would be taken
    ///
    /// eg. if one rule times a user out for 60 seconds and another times out for 120 seconds, there would be one action that times out for 120 seconds
    pub actions: Vec<AutomodAction>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct AutomodMatches {
    /// the original text
    pub text: String,

    /// the sanitized text that was matched against
    pub sanitized_text: String,

    /// the substrings in the input text that matched
    pub matches: Vec<String>,

    /// the keywords in the automod rule that matched
    pub keywords: Vec<String>,

    /// the regexes in the automod rule that matched
    pub regexes: Vec<String>,
    // /// where this piece of text was found
    // pub location: AutomodTextLocation,
}

/// where a piece of text was found
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum AutomodTextLocation {
    /// the user's name
    UserName,

    /// the user's bio (description)
    UserBio,

    /// a room member's nickname
    MemberNickname,

    /// the content of a message that tried to be sent
    MessageContent,

    /// the title of a thread that tried to be created
    ThreadTitle,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum AutomodTrigger {
    /// scan text based on regex. regexes are case insensitive.
    TextRegex {
        /// deny content that matches any of these regexes.
        // max length 32
        deny: Vec<String>,

        /// allow content that matches any of these regexes. overrides deny.
        // max length 32
        allow: Vec<String>,
    },

    /// scan text based on its keywords. automatically adds word boundaries and decancers the string (ie. properly handles unicode lookalikes).
    TextKeywords {
        // max length 32
        keywords: Vec<String>,

        // max length 32
        allow: Vec<String>,
    },

    /// deny text based on links
    TextLinks {
        /// which hostnames to block or allow. works recursively (ie. foo.example.com is blocked if example.com is blocked)
        hostnames: Vec<String>,

        /// whether this is a list of allowed link domains, otherwise this is a blacklist
        whitelist: bool,
    },

    /// a builtin server defined list
    TextBuiltin {
        /// the name of the server defined list
        // NOTE: maybe i want to use an id here instead?
        list: String,
    },

    /// a builtin server defined media scanner
    MediaScan {
        /// the name of a server defined media scanner
        ///
        /// for example, `Nsfw` or `Malware`
        // NOTE: maybe i want to use an id here instead?
        scanner: String,
    },
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum AutomodAction {
    /// block the message from being sent
    Block {
        /// a custom message to show to the user
        // TODO: enforce that this is between 1-256 chars
        message: Option<String>,
    },

    /// timeout a user. not valid for `AutomodTarget::Member`.
    Timeout {
        /// in milliseconds
        duration: u64,
    },

    /// remove a message. unlike Block, removed messages can be allowed/restored by a moderator. not valid for `AutomodTarget::Member`.
    Remove,

    /// send an alert to a channel
    SendAlert {
        /// where to send the alert to
        // TODO: enforce that this channel exists and is a text channel
        // TODO: remove this action when channel is removed
        channel_id: ChannelId,
    },
}
