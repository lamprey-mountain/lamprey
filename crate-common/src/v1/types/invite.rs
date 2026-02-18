use std::fmt;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::util::Time;
use crate::v1::types::{ChannelId, PaginationKey, Role, RoleId, RoomId, UserId};

#[cfg(feature = "serde")]
use crate::v1::types::util::deserialize_sorted;

use super::{Channel, Room, User};

/// a short, unique identifier. knowing the code grants access to the invite's target.
#[derive(Debug, Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema), schema(examples("a1b2c3")))]
pub struct InviteCode(pub String);

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Invite {
    /// the invite code for this invite
    pub code: InviteCode,

    /// where this invite leads
    pub target: InviteTarget,

    /// the user who created this invite
    // #[deprecated = "use creator_id"]
    pub creator: User,

    /// the id of the user who created this invite
    pub creator_id: UserId,

    /// the time when this invite was created
    pub created_at: Time,

    /// the time when this invite will stop working
    pub expires_at: Option<Time>,

    /// a description for this invite
    pub description: Option<String>,

    /// if this invite's code is custom (instead of random)
    // TODO(#263): vanity (custom) invite codes
    pub is_vanity: bool,
    // TODO: maybe enable/disable invites without revoking them?
    // pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct InviteWithMetadata {
    /// the maximum number of times this invite can be used
    pub max_uses: Option<u16>,

    /// the number of time this invite has been used
    pub uses: u64,

    /// the invite this metadata is for
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub invite: Invite,
}

/// where this invite leads
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum InviteTarget {
    /// join a room
    Room {
        /// the room itself
        room: Room,

        /// which channel in the room to open
        channel: Option<Box<Channel>>,

        /// which roles to apply when accepting the invite
        roles: Vec<Role>,
    },

    /// join a group dm
    Gdm { channel: Box<Channel> },

    /// register on this server
    Server,

    /// add this user as a friend
    User { user: Box<User> },
}

/// the type and id of this invite's target
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum InviteTargetId {
    Room {
        room_id: RoomId,
        channel_id: Option<ChannelId>,

        #[cfg_attr(
            feature = "serde",
            serde(default, deserialize_with = "deserialize_sorted")
        )]
        role_ids: Vec<RoleId>,
    },

    Gdm {
        channel_id: ChannelId,
    },

    Server,

    User {
        user_id: UserId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct InvitePatch {
    /// a description for this invite
    pub description: Option<Option<String>>,

    /// the time when this invite will stop working
    pub expires_at: Option<Option<Time>>,

    /// the maximum number of times this invite can be used
    /// be sure to account for existing `uses` and `max_uses` when patching
    pub max_uses: Option<Option<u16>>,

    // can only be used with room invites
    pub role_ids: Option<Vec<RoleId>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct InviteCreate {
    /// a description for this invite
    #[cfg_attr(feature = "utoipa", schema(required = false))]
    pub description: Option<String>,

    /// the time when this invite will stop working
    #[cfg_attr(feature = "utoipa", schema(required = false))]
    pub expires_at: Option<Time>,

    /// the maximum number of times this invite can be used
    /// be sure to account for existing `uses` and `max_uses` when patching
    #[cfg_attr(feature = "utoipa", schema(required = false))]
    pub max_uses: Option<u16>,

    /// which roles to apply when accepting the invite
    #[cfg_attr(feature = "serde", serde(default))]
    pub role_ids: Option<Vec<RoleId>>,
}

impl fmt::Display for InviteCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PaginationKey for InviteCode {
    fn min() -> Self {
        InviteCode("".to_string())
    }

    fn max() -> Self {
        InviteCode("ZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ".to_string())
    }
}

impl From<InviteWithMetadata> for Invite {
    fn from(value: InviteWithMetadata) -> Self {
        value.invite
    }
}

impl InviteWithMetadata {
    pub fn strip_metadata(self) -> Invite {
        self.into()
    }

    pub fn is_dead(&self) -> bool {
        if let Some(max_uses) = self.max_uses {
            if self.uses >= max_uses as u64 {
                return true;
            }
        }
        if let Some(ref expires_at) = self.invite.expires_at {
            if *expires_at < Time::now_utc() {
                return true;
            }
        }
        false
    }
}

impl Invite {
    pub fn new(
        code: InviteCode,
        target: InviteTarget,
        creator: User,
        creator_id: UserId,
        created_at: Time,
        expires_at: Option<Time>,
        description: Option<String>,
        is_vanity: bool,
    ) -> Self {
        Self {
            code,
            target,
            creator,
            creator_id,
            created_at,
            expires_at,
            description,
            is_vanity,
        }
    }
}

impl PartialEq for Invite {
    fn eq(&self, other: &Self) -> bool {
        self.code == other.code
    }
}

impl Eq for Invite {}
