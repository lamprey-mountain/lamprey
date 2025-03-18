use serde::{Deserialize, Serialize};

use url::Url;
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::user_status::Status;
use crate::v1::types::util::{some_option, Diff, Time};
use crate::v1::types::{MediaId, RoomId, ThreadId};

use super::user_config::UserConfig;
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

    #[serde(flatten)]
    pub user_type: UserType,

    pub state: UserState,
    pub state_updated_at: Time,
    pub status: Status,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct UserWithPrivate {
    #[serde(flatten)]
    pub inner: User,
    pub config: UserConfig,
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

    #[serde(flatten)]
    pub user_type: UserType,
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

    /// automated account
    Bot {
        /// who/what has control over this bot
        #[serde(flatten)]
        owner: BotOwner,

        /// who can use the bot
        visibility: BotVisibility,

        /// enables managing Puppet users
        is_bridge: bool,
        // do i really need all these urls?
        // url_terms_of_service: Option<Url>,
        // url_privacy_policy: Option<Url>,
        // url_help_docs: Vec<Url>,
        // url_main_site: Vec<Url>,
        // url_interactions: Vec<Url>, // webhook
    },

    /// a special type of bot designed to represent a user on another platform
    // maybe all bots/user types can create puppets, but there's an extra permission for bridging?
    Puppet {
        /// the user who created this puppet
        owner_id: UserId,

        /// what platform this puppet is connected to
        external_platform: ExternalPlatform,

        /// an opaque identifier from the other platform
        #[cfg_attr(
            feature = "utoipa",
            schema(required = false, min_length = 1, max_length = 8192)
        )]
        external_id: String,

        /// a url on the other platform that this account can be reached at
        external_url: Option<Url>,

        /// makes two users be considered the same user, for importing
        /// stuff from other platforms
        /// can you alias to another puppet?
        alias_id: Option<UserId>,
    },

    /// system/service account
    System,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "owner_type")]
pub enum BotOwner {
    /// owned by a thread (ie. for webhooks)
    Thread { thread_id: ThreadId },

    /// owned by a room (one off room-specific bot account)
    Room { room_id: RoomId },

    /// owned by a user (most bots)
    User { user_id: UserId },

    /// an official system bot
    ///
    /// avoid using the system user directly since its effectively root. create
    /// Server bots instead.
    Server,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum BotVisibility {
    /// only the creator can use the bot
    #[default]
    Private,

    /// anyone can use the bot
    Public {
        /// anyone can search for and find this; otherwise, this is unlisted
        is_discoverable: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(untagged)]
pub enum ExternalPlatform {
    /// discord
    Discord,

    /// some other platform
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum UserState {
    Active,
    Suspended,
    Deleted,
}

impl Diff<User> for UserPatch {
    fn changes(&self, other: &User) -> bool {
        self.name.changes(&other.name)
            || self.description.changes(&other.description)
            || self.avatar.changes(&other.avatar)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Relationship {
    /// whatever you want to write
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 4096))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 4096)))]
    pub note: Option<String>,

    /// relationship with other user
    pub relation: Option<RelationshipType>,

    /// personal petname for this user
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub petname: Option<String>,

    #[serde(flatten)]
    pub ignore: Option<Ignore>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RelationshipWithUserId {
    #[serde(flatten)]
    pub inner: Relationship,
    pub user_id: UserId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct RelationshipPatch {
    /// whatever you want to write
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 4096))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 4096)))]
    #[serde(default, deserialize_with = "some_option")]
    pub note: Option<Option<String>>,

    /// relationship with other user
    #[serde(default, deserialize_with = "some_option")]
    pub relation: Option<Option<RelationshipType>>,

    /// personal petname for this user
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    #[serde(default, deserialize_with = "some_option")]
    pub petname: Option<Option<String>>,

    #[cfg_attr(feature = "utoipa", schema(required = false))]
    #[serde(default, flatten, deserialize_with = "some_option")]
    pub ignore: Option<Option<Ignore>>,
}

/// how a user is ignoring another user
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "ignore")]
pub enum Ignore {
    Until { ignore_until: Time },
    Forever,
}

/// a relationship between two users
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum RelationshipType {
    /// friends :D
    Friend,

    /// outgoing friend request
    Outgoing,

    /// incoming friend request
    Incoming,

    /// blocked
    Block,
}

impl UserType {
    pub fn can_create(&self, other: &UserType) -> bool {
        match (self, other) {
            // allowed as a fast path (maybe get to skip some captchas)
            // though this might be abusable
            (UserType::Default, UserType::Default) => true,

            // TODO: ensure that user_id is correct
            // users can create bots
            (UserType::Default, UserType::Bot { .. }) => true,

            // if you want to use a bridge, create a bot
            (UserType::Default, UserType::Puppet { .. }) => false,

            // for bridging
            // (UserType::Bot { is_bridge, .. }, UserType::Puppet { .. }) => *is_bridge,
            (UserType::Bot { .. }, UserType::Puppet { .. }) => true,

            // doesn't really make sense for a bot to be able to create more non-puppet users
            // maybe creating accounts for users, idk
            (UserType::Bot { .. }, UserType::Default | UserType::Bot { .. }) => false,

            // puppets cant create new accounts, but their owner can
            (UserType::Puppet { .. }, _) => false,

            // system user is root and can do anything
            (UserType::System, _) => true,

            // there is only one system user
            (_, UserType::System) => false,
        }
    }
}

impl Diff<Relationship> for RelationshipPatch {
    fn changes(&self, other: &Relationship) -> bool {
        self.note.changes(&other.note)
            || self.relation.changes(&other.relation)
            || self.petname.changes(&other.petname)
            || self.ignore.changes(&other.ignore)
    }
}
