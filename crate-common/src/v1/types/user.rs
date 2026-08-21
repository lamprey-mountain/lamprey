use lamprey_macros::record;
use url::Url;

use crate::v1::types::MediaId;
use crate::v1::types::error::{ApiError, ErrorCode};
use crate::v1::types::federation::Remote;
use crate::v1::types::preferences::PreferencesUser;
use crate::v1::types::presence::Presence;
use crate::v1::types::util::{Diff, Time};

#[cfg(feature = "serde")]
use crate::v1::types::util::some_option;

use super::email::EmailInfo;
use super::preferences::PreferencesGlobal;
use super::{ApplicationId, ChannelId, RoomId, UserId, UserVerId};

// TODO: dedicated user.created_at field instead of parsing user.id
#[record]
pub struct User {
    pub id: UserId,
    pub version_id: UserVerId,

    #[schema(min_length = 1, max_length = 64)]
    #[validate(length(min = 1, max = 64))]
    pub name: String,

    // TODO: rename to bio?
    #[schema(required = false, min_length = 1, max_length = 8192)]
    #[validate(length(min = 1, max = 8192))]
    pub description: Option<String>,

    pub avatar: Option<MediaId>,
    pub banner: Option<MediaId>,

    /// whether this user is a bot
    pub bot: bool,

    /// whether this user is an official system user
    pub system: bool,

    // skip serializing if is_none
    pub puppet: Option<Puppet>,

    // skip serializing if is_none
    pub webhook: Option<UserWebhook>,

    // skip serializing if is_none; only return for admins
    pub suspended: Option<Suspended>,

    pub presence: Presence,

    // ...remove? unsure how this will work.
    pub registered_at: Option<Time>,

    // skip serializing if is_none; only return for admins
    pub deleted_at: Option<Time>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub emails: Option<Vec<EmailInfo>>,
    pub preferences: Option<PreferencesUser>,
    // #[ validate(length(min = 1, max = 16))]
    // pub fields: Vec<UserField>,
    /// whether this user is considered to have mutifactor authentication enabled on their account
    ///
    /// this allows using certain restricted endpoints if a room requires it via `security.require_mfa`
    pub has_mfa: Option<bool>,

    #[cfg(any())]
    /// public connections on this user that you can view
    pub connections: Vec<Connection>,

    pub remote: Option<Remote<UserId>>,
}

#[record]
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

#[record]
pub struct Suspended {
    pub created_at: Time,
    pub expires_at: Option<Time>,
    pub reason: Option<String>,
}

/// represents a user on another platform
#[record]
pub struct Puppet {
    /// the user who created this puppet
    pub owner_id: ApplicationId,

    /// an opaque identifier from the other platform
    #[schema(required = false, min_length = 1, max_length = 8192)]
    pub external_id: String,

    /// a url on the other platform that this account can be reached at
    pub external_url: Option<Url>,

    /// makes two users be considered the same user, for importing
    /// stuff from other platforms
    /// can you alias to another puppet?
    pub alias_id: Option<UserId>,
}

#[record]
pub struct UserWithPrivate {
    #[serde(flatten)]
    pub inner: User,
    pub config: PreferencesGlobal,
}

#[record]
#[derive(PartialEq, Eq)]
pub struct UserCreate {
    #[schema(min_length = 1, max_length = 64)]
    #[validate(length(min = 1, max = 64))]
    pub name: String,

    #[schema(required = false, min_length = 1, max_length = 8192)]
    #[validate(length(min = 1, max = 8192))]
    pub description: Option<String>,
}

#[record]
#[derive(PartialEq, Eq)]
pub struct PuppetCreate {
    /// display name
    #[schema(min_length = 1, max_length = 64)]
    #[validate(length(min = 1, max = 64))]
    pub name: String,

    /// about/bio
    #[schema(required = false, min_length = 1, max_length = 8192)]
    #[validate(length(min = 1, max = 8192))]
    pub description: Option<String>,

    /// if this is a remote bot
    pub bot: bool,

    /// if this is for the service itself. usually paired with bot: true
    pub system: bool,
}

#[record]
#[derive(Default, PartialEq, Eq, lamprey_macros::Diff)]
pub struct UserPatch {
    #[schema(required = false, min_length = 1, max_length = 64)]
    #[validate(length(min = 1, max = 64))]
    pub name: Option<String>,

    #[schema(required = false, min_length = 1, max_length = 8192)]
    #[validate(length(min = 1, max = 8192))]
    #[serde(default, deserialize_with = "some_option")]
    pub description: Option<Option<String>>,

    #[serde(default, deserialize_with = "some_option")]
    pub avatar: Option<Option<MediaId>>,

    #[serde(default, deserialize_with = "some_option")]
    pub banner: Option<Option<MediaId>>,
}

#[record]
#[derive(Default, PartialEq, Eq)]
pub struct Relationship {
    /// your relationship with this other user
    pub relation: Option<RelationshipType>,

    #[serde(flatten)]
    pub ignore: Option<Ignore>,
}

#[record]
#[derive(Default, PartialEq, Eq)]
pub struct RelationshipWithUserId {
    #[serde(flatten)]
    pub inner: Relationship,
    pub user_id: UserId,
}

#[record]
pub struct UserWithRelationship {
    #[serde(flatten)]
    pub inner: User,
    pub relationship: Relationship,
}

#[record]
#[derive(PartialEq, Eq, Diff)]
pub struct RelationshipPatch {
    /// relationship with other user
    #[serde(default, deserialize_with = "some_option")]
    pub relation: Option<Option<RelationshipType>>,

    #[schema(required = false)]
    #[serde(default, flatten, deserialize_with = "some_option")]
    pub ignore: Option<Option<Ignore>>,
}

/// how a user is ignoring another user
#[record]
#[derive(PartialEq, Eq)]
pub struct Ignore {
    pub until: Option<Time>,
}

/// a relationship between two users
#[record]
#[derive(PartialEq, Eq)]
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

// TODO: remove?
#[record]
#[derive(PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UserListFilter {
    Guest,
    Registered,
    Bot,
    Puppet,
}

#[record]
#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(utoipa::IntoParams))]
#[serde(rename_all = "snake_case")]
pub struct UserListParams {
    pub filter: Option<UserListFilter>,
}

#[record]
#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(utoipa::IntoParams))]
pub struct SuspendRequest {
    pub expires_at: Option<Time>,
}

impl User {
    /// check if a user is suspended or not
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

    /// ensure that this user is not suspended, returning an Err if they are
    pub fn ensure_unsuspended(&self) -> Result<(), ApiError> {
        if self.is_suspended() {
            Err(ApiError::from_code(ErrorCode::UserSuspended))
        } else {
            Ok(())
        }
    }

    /// whether a direct message can be created with this user
    pub fn can_dm(&self) -> bool {
        self.webhook.is_none()
    }

    /// whether a friend request can be sent to this user
    pub fn can_friend(&self) -> bool {
        self.webhook.is_none() && !self.bot && self.puppet.is_none()
    }

    /// whether auth state can be updated for this user
    pub fn can_update_auth(&self) -> bool {
        self.webhook.is_none() && !self.bot && self.puppet.is_none() && self.remote.is_none()
    }
}
