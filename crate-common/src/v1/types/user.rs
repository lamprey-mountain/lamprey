use serde::{Deserialize, Serialize};

use url::Url;
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::error::Error;
use crate::v1::types::user_status::Status;
use crate::v1::types::util::{some_option, Diff, Time};
use crate::v1::types::MediaId;

use super::user_config::UserConfig;
use super::{UserId, UserVerId};

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    pub avatar: Option<MediaId>,
    pub bot: Option<Bot>,
    pub system: bool,
    pub puppet: Option<Puppet>,
    pub suspended: Option<Suspended>,
    // TODO: rename to presence
    pub status: Status,
    pub registered_at: Option<Time>,
    pub deleted_at: Option<Time>,
    // pub email: Vec<Email>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Suspended {
    pub created_at: Time,
    pub expires_at: Option<Time>,
    pub reason: Option<String>,
}

/// represents a user on another platform
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Puppet {
    /// the user who created this puppet
    pub owner_id: UserId,

    /// what platform this puppet is connected to
    pub external_platform: ExternalPlatform,

    /// an opaque identifier from the other platform
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 8192)
    )]
    pub external_id: String,

    /// a url on the other platform that this account can be reached at
    pub external_url: Option<Url>,

    /// makes two users be considered the same user, for importing
    /// stuff from other platforms
    /// can you alias to another puppet?
    pub alias_id: Option<UserId>,
}

/// a special type of bot designed to represent a user on another platform
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Bot {
    /// who has control over this bot
    pub owner_id: UserId,

    /// who can use the bot
    pub access: BotAccess,

    /// enables managing Puppet users
    // maybe all bots/user types can create puppets, but there's an extra permission for bridging?
    pub is_bridge: bool,
    // do i really need all these urls properties, or can i get away with a vec?
    // url_terms_of_service: Option<Url>,
    // url_privacy_policy: Option<Url>,
    // url_help_docs: Vec<Url>,
    // url_main_site: Vec<Url>,
    // url_interactions: Vec<Url>, // webhook
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

pub struct BotCreate;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct PuppetCreate {
    /// display name
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: String,

    /// about/bio
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 8192)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub description: Option<String>,

    /// if this is a remote bot
    pub bot: bool,

    /// if this is for the service itself. usually paired with bot: true
    pub system: bool,
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

// // TODO: later
// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// #[serde(tag = "owner_type")]
// pub enum BotOwner {
//     /// owned by a thread (ie. for webhooks)
//     Thread { thread_id: ThreadId },

//     /// owned by a room (one off room-specific bot account)
//     Room { room_id: RoomId },

//     /// owned by a user (most bots)
//     User { user_id: UserId },

//     /// an official system bot
//     ///
//     /// avoid using the system user directly since its effectively root. create
//     /// Server bots instead.
//     Server,
// }

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum BotAccess {
    /// only the creator can use the bot
    #[default]
    Private,

    /// anyone can use the bot
    Public {
        /// anyone can search for and find this; otherwise, this is unlisted
        is_discoverable: bool,
    },
}

// TODO: move to bridge info rather than per puppet?
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

    /// your relationship with this other user
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct UserWithRelationship {
    #[serde(flatten)]
    pub inner: User,
    pub relationship: Relationship,
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

impl Diff<Relationship> for RelationshipPatch {
    fn changes(&self, other: &Relationship) -> bool {
        self.note.changes(&other.note)
            || self.relation.changes(&other.relation)
            || self.petname.changes(&other.petname)
            || self.ignore.changes(&other.ignore)
    }
}

impl User {
    pub fn is_suspended(&self) -> bool {
        if let Some(s) = &self.suspended {
            if s.expires_at.is_some_and(|t| *t < *Time::now_utc()) {
                false
            } else {
                true
            }
        } else {
            false
        }
    }

    pub fn ensure_unsuspended(&self) -> Result<(), Error> {
        if self.is_suspended() {
            Err(Error::UserSuspended)
        } else {
            Ok(())
        }
    }
}
