use crate::v1::types::{reaction::ReactionKey, AutomodRuleId, RoleId, RoomId, ThreadId};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct AutomodRule {
    pub id: AutomodRuleId,
    pub room_id: RoomId,
    #[schema(max_length = 64)]
    pub name: String,
    pub enabled: bool,
    pub trigger: AutomodTrigger,
    #[schema(max_items = 8)]
    pub actions: Vec<AutomodAction>,
    pub except_roles: Vec<RoleId>,
    pub except_threads: Vec<ThreadId>,
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
    pub except_threads: Vec<ThreadId>,
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
    pub except_threads: Option<Vec<ThreadId>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum AutomodTrigger {
    MessageRegex {
        // max length 32
        deny: Vec<String>,

        // max length 32
        allow: Vec<String>,
    },

    MessageKeywords {
        // max length 32
        words: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum AutomodAction {
    Block,

    Timeout {
        /// in milliseconds
        duration: u64,
    },

    Remove,

    SendAlert {
        thread_id: ThreadId,
    },

    React {
        reaction: ReactionKey,
    },
}
