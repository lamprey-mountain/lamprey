#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use url::Url;

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::error::{ApiError, ErrorCode};
use crate::v1::types::presence::Presence;
use crate::v1::types::search::Order;
use crate::v1::types::user_config::PreferencesUser;
use crate::v1::types::util::{some_option, Diff, Time};
use crate::v1::types::{MediaId, RoleId};

use super::email::EmailInfo;
use super::user_config::PreferencesGlobal;
use super::{ApplicationId, ChannelId, RoomId, UserId, UserVerId};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct User {
    pub id: UserId,
    pub version_id: UserVerId,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: String,

    // TODO: rename to bio?
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 8192)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub description: Option<String>,

    pub avatar: Option<MediaId>,
    pub banner: Option<MediaId>,

    /// whether this user is a bot
    pub bot: bool,

    /// whether this user is an official system user
    pub system: bool,

    pub puppet: Option<Puppet>,
    pub webhook: Option<UserWebhook>,
    pub suspended: Option<Suspended>,
    pub presence: Presence,
    pub registered_at: Option<Time>,
    pub deleted_at: Option<Time>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emails: Option<Vec<EmailInfo>>,
    pub user_config: Option<PreferencesUser>,
    // #[cfg_attr(feature = "validator", validate(length(min = 1, max = 16)))]
    // pub fields: Vec<UserField>,
    /// whether this user is considered to have mutifactor authentication enabled on their account
    ///
    /// this allows using certain restricted endpoints if a room requires it via `security.require_mfa`
    pub has_mfa: Option<bool>,

    #[cfg(any())]
    /// public connections on this user that you can view
    pub connections: Vec<Connection>,

    #[cfg(any())]
    pub remote: Option<Remote>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct UserWebhook {
    pub room_id: Option<RoomId>,
    pub channel_id: ChannelId,
    pub creator_id: UserId,
}

// #[derive(Debug, Clone, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// pub struct UserField {
//     #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
//     #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
//     pub key: String,

//     #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 2048))]
//     #[cfg_attr(feature = "validator", validate(length(min = 1, max = 2048)))]
//     pub value: String,

//     // TODO: skip_serializing_if false
//     /// if this url is verified?
//     pub verified: bool,
// }

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
    pub owner_id: ApplicationId,

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct UserWithPrivate {
    #[serde(flatten)]
    pub inner: User,
    pub config: PreferencesGlobal,
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

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    #[serde(default, deserialize_with = "some_option")]
    pub banner: Option<Option<MediaId>>,
}

impl Diff<User> for UserPatch {
    fn changes(&self, other: &User) -> bool {
        self.name.changes(&other.name)
            || self.description.changes(&other.description)
            || self.avatar.changes(&other.avatar)
            || self.banner.changes(&other.banner)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Relationship {
    /// your relationship with this other user
    pub relation: Option<RelationshipType>,

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
    /// relationship with other user
    #[serde(default, deserialize_with = "some_option")]
    pub relation: Option<Option<RelationshipType>>,

    #[cfg_attr(feature = "utoipa", schema(required = false))]
    #[serde(default, flatten, deserialize_with = "some_option")]
    pub ignore: Option<Option<Ignore>>,
}

/// how a user is ignoring another user
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Ignore {
    pub until: Option<Time>,
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
        self.relation.changes(&other.relation) || self.ignore.changes(&other.ignore)
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

    pub fn ensure_unsuspended(&self) -> Result<(), ApiError> {
        if self.is_suspended() {
            Err(ApiError::from_code(ErrorCode::UserSuspended))
        } else {
            Ok(())
        }
    }
}

// TODO: remove?
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum UserListFilter {
    Guest,
    Registered,
    Bot,
    Puppet,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
#[serde(rename_all = "snake_case")]
pub struct UserListParams {
    pub filter: Option<UserListFilter>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
pub struct UserSearch {
    /// whether to only return bots or only return non-bots.
    ///
    /// defaults to allowing both.
    pub bot: Option<bool>,

    /// whether to only return puppets or only return non-puppets.
    ///
    /// defaults to allowing both.
    pub puppet: Option<bool>,

    /// whether to only return guests (non registered users) or only return non-guests.
    ///
    /// defaults to allowing both.
    pub guests: Option<bool>,

    /// whether to only return suspended users or only return non-suspended users.
    ///
    /// defaults to allowing both.
    pub suspended: Option<bool>,

    /// whether to only return deleted users or only return non-deleted users.
    ///
    /// defaults to only non deleted users.
    // FIXME: defaul to Some(false)
    pub deleted: Option<bool>,

    /// filter by user name, description, and id
    // NOTE: impl this with ILIKE, similarly to room member filtering
    pub query: Option<String>,

    /// include users who have these roles in the server room
    pub server_role_id: Vec<RoleId>,

    /// include users who are members of these rooms
    pub member_of_room_id: Vec<RoomId>,

    pub sort_order: Order,
    pub sort_field: UserSearchSortField,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum UserSearchSortField {
    /// the user name
    Name,

    /// the user's created_at (aka id)
    Created,

    /// when the user was registered
    Registered,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
pub struct SuspendRequest {
    pub expires_at: Option<Time>,
}

impl User {
    /// whether a direct message can be created with this user
    pub fn can_dm(&self) -> bool {
        self.webhook.is_none()
    }

    /// whether a friend request can be sent to this user
    pub fn can_friend(&self) -> bool {
        self.webhook.is_none() && !self.bot && self.puppet.is_none()
    }
}
