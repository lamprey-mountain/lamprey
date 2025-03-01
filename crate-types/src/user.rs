use serde::{Deserialize, Serialize};

use url::Url;
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::user_status::Status;
use crate::util::{some_option, Diff};
use crate::MediaId;

use super::util::deserialize_default_true;
use super::{UserId, UserVerId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct User {
    pub id: UserId,
    pub version_id: UserVerId,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: String,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 8192)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub description: Option<String>,

    // NOTE: do i want to resolve media here?
    // it's nice to have but is redundant, immutable, and common data
    pub avatar: Option<MediaId>,

    // email: Option<String>,
    #[serde(flatten)]
    pub user_type: UserType,

    pub state: UserState,

    pub status: Status,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct UserCreate {
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: String,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 8192)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub description: Option<String>,

    // TODO: replace with UserCreateType
    #[serde(deserialize_with = "deserialize_default_true")]
    pub is_bot: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum UserCreateType {
    Default,
    Bot {
        // /// what this bot has access to
        // scope: BotScope,

        // might be simplified?
        // url_terms_of_service: Option<Url>,
        // url_privacy_policy: Option<Url>,
        // url_help_docs: Vec<Url>,
        // url_main_site: Vec<Url>,
        // url_interactions: Vec<Url>, // webhook
        // visibility: BotVisibility,
    },
    Puppet {
        /// what platform this puppet is connected to
        external_platform: ExternalPlatform,

        /// an opaque identifier
        // TODO: validate lengths
        // #[cfg_attr(
        //     feature = "utoipa",
        //     schema(required = false, min_length = 1, max_length = 8192)
        // )]
        external_id: String,

        /// a url on the other platform that this account can be reached at
        external_url: Option<Url>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct UserPatch {
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 64)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: Option<String>,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 8192)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    #[serde(default, deserialize_with = "some_option")]
    pub description: Option<Option<String>>,

    #[serde(default, deserialize_with = "some_option")]
    pub avatar: Option<Option<MediaId>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum UserType {
    /// a normal user
    Default,

    /// makes two users be considered the same user
    Alias {
        /// this user should be considered the same as the one at alias_id
        /// mainly intended for bridged users
        /// maybe it should even redirect by default?
        /// can you alias to another alias?
        alias_id: UserId,
    },

    /// automated account
    Bot {
        /// the user who created this bot
        owner_id: UserId,
        // /// what this bot has access to
        // scope: BotScope,

        // might be simplified?
        // url_terms_of_service: Option<Url>,
        // url_privacy_policy: Option<Url>,
        // url_help_docs: Vec<Url>,
        // url_main_site: Vec<Url>,
        // url_interactions: Vec<Url>, // webhook
        // visibility: BotVisibility,
    },

    /// a special type of bot designed to represent a user on another platform
    // maybe all bots/user types can create puppets, but there's an extra permission for bridging?
    Puppet {
        /// the user who created this puppet
        owner_id: UserId,

        /// what platform this puppet is connected to
        external_platform: ExternalPlatform,

        /// an opaque identifier
        external_id: String,

        /// a url on the other platform that this account can be reached at
        external_url: Option<Url>,
    },

    /// system/service account
    System,
}

// TODO: add platforms
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ExternalPlatform {
    /// some other platform
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum UserState {
    Active,
    // maybe different "trust levels" for antispam
    // Untrusted,
    // Trusted,
    // Verified,
    Suspended,
    Deleted,
}

/// what this bot can access
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum BotScope {
    /// this bot can be used everywhere (most bots)
    Global,

    /// this bot can only access a single room (probably somewhat rare?)
    Room { room_id: crate::RoomId },

    /// this bot can only access a single thread (ie. webhooks)
    Thread { room_id: crate::ThreadId },
}

impl Diff<User> for UserPatch {
    fn changes(&self, other: &User) -> bool {
        self.name.changes(&other.name)
            || self.description.changes(&other.description)
            || self.avatar.changes(&other.avatar)
    }
}

/// data private to the user
pub struct UserPrivate {
    pub note: Option<String>,
    pub relation: Option<Relationship>,
    pub ignore: Option<Ignore>,
}

/// how a user is ignoring another user
pub enum Ignore {
    Timed { ignore_until: time::OffsetDateTime },
    Forever,
}

/// a relationship between two users
pub enum Relationship {
    RequestSend,
    RequestRecv,
    Friend,
    Block,
}

// #[derive(Debug, Default)]
// pub enum BotVisibility {
//     #[default]
//     Private,
//     Public,
//     Discoverble,
// }

// // maybe could be merged with scope
// pub enum BotOwner {
//     User(UserId),
//     Room(RoomId),
//     Thread(ThreadId),
//     // replaces other UserTypes ..?
//     Server,
//     Puppet(PuppetInfo),
// }
