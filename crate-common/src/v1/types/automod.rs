use crate::v1::types::{AutomodRuleId, ChannelId, MessageId, RoleId, RoomId, UserId};

use lamprey_macros::record;

#[cfg(feature = "validator")]
use validator::Validate;

// TODO: better doc comments

/// an auto moderation rule for a room
// TODO(?): maybe allow multiple triggers per rule
// "execute this rule when ANY of these triggers match"
#[record]
pub struct AutomodRule {
    pub id: AutomodRuleId,
    pub room_id: RoomId,

    /// a human readable label for this rule
    #[schema(max_length = 64)]
    pub name: String,

    /// whether any actions occur when this rule is triggered
    pub enabled: bool,

    pub trigger: AutomodTrigger,

    pub target: AutomodTarget,

    /// when executed, do ALL of these actions
    #[schema(max_items = 8)]
    pub actions: Vec<AutomodAction>,

    /// what roles should be exempt from this rule. users with RoomManage are always exempt.
    pub except_roles: Vec<RoleId>,

    /// what channels should be exempt from this rule.
    pub except_channels: Vec<ChannelId>,

    /// if nsfw channels should be exempt from this rule.
    pub except_nsfw: bool,

    /// whether this rule should affect everyone. actions aren't necessarily executed (eg. admins wont be timed out)
    pub include_everyone: bool,
}

#[cfg(feature = "serde")]
fn true_fn() -> bool {
    true
}

#[record]
pub struct AutomodRuleCreate {
    #[schema(max_length = 64)]
    #[validate(length(min = 1, max = 64))]
    pub name: String,

    // FIXME: support this
    #[serde(default = "true_fn")]
    pub enabled: bool,

    pub target: AutomodTarget,

    #[validate(nested)]
    pub trigger: AutomodTrigger,

    #[schema(max_items = 8)]
    #[validate(length(min = 1, max = 8), nested)]
    pub actions: Vec<AutomodAction>,

    #[serde(default)]
    pub except_roles: Vec<RoleId>,

    #[serde(default)]
    pub except_channels: Vec<ChannelId>,

    #[serde(default)]
    pub except_nsfw: bool,

    #[serde(default)]
    pub include_everyone: bool,
}

#[record]
pub struct AutomodRuleUpdate {
    #[schema(max_length = 64)]
    #[validate(length(min = 1, max = 64))]
    pub name: Option<String>,

    pub enabled: Option<bool>,

    pub target: Option<AutomodTarget>,

    #[validate(nested)]
    pub trigger: Option<AutomodTrigger>,

    #[schema(max_items = 8)]
    #[validate(length(min = 1, max = 8), nested)]
    pub actions: Option<Vec<AutomodAction>>,

    pub except_roles: Option<Vec<RoleId>>,
    pub except_channels: Option<Vec<ChannelId>>,
    pub except_nsfw: Option<bool>,
    pub include_everyone: Option<bool>,
}

/// minimal version of AutomodRule to prevent leaking the rule trigger
#[record]
pub struct AutomodRuleStripped {
    pub id: AutomodRuleId,
    pub name: String,
    pub enabled: bool,
    pub target: AutomodTarget,
}

impl From<AutomodRule> for AutomodRuleStripped {
    fn from(rule: AutomodRule) -> Self {
        Self {
            id: rule.id,
            name: rule.name,
            enabled: rule.enabled,
            target: rule.target,
        }
    }
}

/// what this rule should be evaluated on
#[record]
#[derive(Copy, PartialEq, Eq)]
pub enum AutomodTarget {
    /// messages, threads, voice statuses
    Content,

    /// user names, bios, and nicknames
    Member,
}

// NOTE: may be fired multiple times for a piece of content, if there are multiple rules which target it
#[record]
pub struct AutomodRuleExecution {
    /// the id of the room that this execution happened in
    pub room_id: RoomId,

    /// the rule that was executed
    pub rule: AutomodRule,

    /// the user who triggered this execution
    pub user_id: UserId,

    /// the channel this happened in (for messages)
    pub channel_id: Option<ChannelId>,

    /// the message this matched (excluded for Block)
    pub message_id: Option<MessageId>,

    /// the id of any automod execution message that was sent due to a SendAlert action
    pub alert_message_id: Vec<MessageId>,

    /// the content that was matched
    pub matches: AutomodMatch,

    /// deduplicated list of all of the actions that were taken
    pub actions: Vec<AutomodAction>,
}

/// request body for an automod test request
#[record]
pub struct AutomodRuleTestRequest {
    /// the text to attempt to scan
    pub text: String,

    /// the target to test this as
    pub target: AutomodTarget,
}

/// response body for an automod test request
#[record]
pub struct AutomodRuleTest {
    /// the rules that matched the text
    pub rules: Vec<AutomodRule>,

    /// the content that was matched
    pub matches: Option<AutomodMatch>,

    /// deduplicated list of all of the actions that would be taken
    ///
    /// eg. if one rule times a user out for 60 seconds and another times out for 120 seconds, there would be one action that times out for 120 seconds
    pub actions: Vec<AutomodAction>,
}

/// matches found in a piece of text
#[record]
pub struct AutomodMatch {
    /// the original text
    pub text: String,

    /// the sanitized text that was matched against
    // NOTE: lamprey uses the decancer crate internally
    pub sanitized_text: String,

    /// each individual match
    pub fragments: Vec<AutomodMatchFragment>,

    /// where this piece of text was found
    pub location: AutomodTextLocation,
}

/// a fragment of text that matched
#[record]
pub struct AutomodMatchFragment {
    /// the substring in the input text that matched
    pub text: String,

    /// the substring in the sanitized input text that matched
    pub sanitized_text: String,

    pub start: usize,
    pub end: usize,

    #[serde(flatten)]
    pub kind: AutomodMatchKind,
}

#[record]
#[serde(tag = "matcher")]
pub enum AutomodMatchKind {
    Keyword { keywords: String },
    Regex { regex: String },
}

/// where a piece of text was found
#[record]
#[derive(PartialEq, Eq, Hash)]
// #[serde(tag = "type")]
pub enum AutomodTextLocation {
    /// the user's name
    UserName,

    /// the user's bio (description)
    UserBio,

    /// a room member's nickname
    MemberNickname,

    /// a room member's description/bio/note
    MemberDescription,

    /// the content of a message
    MessageContent,

    /// the title of a thread
    ThreadTitle,

    /// the topic of a thread
    ThreadTopic,

    /// the title of an embed
    EmbedTitle,

    /// the description of an embed
    EmbedDescription,

    /// the name of an embed author
    EmbedAuthorName,

    /// the url of an embed author
    EmbedAuthorUrl,

    /// the url of an embed
    EmbedUrl,

    /// a test scan
    Test,
}

/// where a piece of media was found
#[record]
#[derive(PartialEq, Eq, Hash)]
pub enum AutomodMediaLocation {
    /// the user's avatar
    UserAvatar,

    /// the user's banner
    UserBanner,

    /// the user's bio (description)
    UserBio,

    /// a message's attachment
    MessageAttachment,

    // TODO: varients for embed media fields
    /// a test scan
    Test,
}

#[record]
#[serde(tag = "type")]
pub enum AutomodTrigger {
    /// scan text based on regex. regexes are case insensitive.
    TextRegex {
        /// deny content that matches any of these regexes.
        // max length 32
        deny: Vec<String>,

        /// allow content that matches any of these regexes. overrides deny.
        // max length 32
        allow: Vec<String>,
        // maybe merge TextKeywords and TextRegex into TextMatch?
        // regex_deny: Vec<String>,
        // regex_allow: Vec<String>,
        // keywords_deny: Vec<String>,
        // keywords_allow: Vec<String>,
    },

    /// scan text based on its keywords. automatically adds word boundaries and decancers the string (ie. properly handles unicode lookalikes).
    TextKeywords {
        // max length 32
        // TODO: rename to deny in api
        #[serde(rename = "keywords")]
        deny: Vec<String>,

        // max length 32
        // probably not useful?
        allow: Vec<String>,
    },

    /// deny text based on links
    TextLinks {
        /// which hostnames to block or allow. works recursively (ie. foo.example.com is blocked if example.com is blocked)
        hostnames: Vec<String>,

        /// whether this is a list of allowed link domains, otherwise this is a blacklist
        whitelist: bool,
    },

    // TODO: redo TextLinks
    // /// target text containing links
    // ///
    // /// - `example.com` blocks the domain `example.com` as well as every subdomain recursively (eg. `foo.example.com`, `bar.foo.example.com`)
    // /// - `*.example.com` blocks subdomains but not `example.com` itself
    // /// - allows always override denies
    // /// - use single `*` to match everything. this is useful in `deny` to use this as a whitelist/allowlist
    // TextLinks {
    //     /// which hostnames to deny
    //     deny: Vec<String>,
    //
    //     /// which hostnames to allow
    //     allow: Vec<String>,
    // },
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

// TODO: split apart Block -> Quarantine for members?
// TODO: separate SendAlert for members? make each action correspond with exactly one target?
#[record]
#[serde(tag = "type")]
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

#[cfg(feature = "validator")]
mod val {
    use validator::{Validate, ValidateLength, ValidationError, ValidationErrors};

    use super::{AutomodAction, AutomodTrigger};

    impl Validate for AutomodTrigger {
        fn validate(&self) -> Result<(), ValidationErrors> {
            let mut errors = ValidationErrors::new();

            match self {
                AutomodTrigger::TextRegex { deny, allow } => {
                    for (i, re) in deny.iter().enumerate() {
                        if regex::Regex::new(re).is_err() {
                            let mut err = ValidationError::new("invalid_regex");
                            err.add_param("index".into(), &serde_json::json!(i));
                            err.add_param("pattern".into(), re);
                            errors.add("deny", err);
                        }
                    }
                    for (i, re) in allow.iter().enumerate() {
                        if regex::Regex::new(re).is_err() {
                            let mut err = ValidationError::new("invalid_regex");
                            err.add_param("index".into(), &serde_json::json!(i));
                            err.add_param("pattern".into(), re);
                            errors.add("allow", err);
                        }
                    }
                }
                _ => {}
            }

            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors)
            }
        }
    }

    impl Validate for AutomodAction {
        fn validate(&self) -> Result<(), ValidationErrors> {
            let mut errors = ValidationErrors::new();

            match self {
                AutomodAction::Block { message } => {
                    if let Some(m) = message {
                        if !m.validate_length(Some(1), Some(256), None) {
                            let mut err = ValidationError::new("length");
                            err.add_param("min".into(), &serde_json::json!(1));
                            err.add_param("max".into(), &serde_json::json!(256));
                            errors.add("message", err);
                        }
                    }
                }
                _ => {}
            }

            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors)
            }
        }
    }
}
