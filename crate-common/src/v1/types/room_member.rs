use lamprey_macros::record;

use super::{RoleId, RoomId, User, UserId};

use crate::v1::types::{
    InviteCode,
    federation::Hostname,
    util::{Diff, Time},
};

#[cfg(feature = "serde")]
use crate::v1::types::util::{deserialize_sorted, deserialize_sorted_option, some_option};

// TODO: move to utils
fn bool_true() -> bool {
    true
}

/// a member in a room
#[record]
pub struct RoomMember {
    pub user_id: UserId,
    pub room_id: RoomId,
    // TODO: split out everything below into RoomMemberInfo? (or some better name)
    /// When this member joined the room
    pub joined_at: Time,

    /// aka nickname
    // TODO: rename to `nick`
    pub override_name: Option<String>,

    /// like nickname, but for your description/bio/about
    // TODO: remove. maybe replace with a room-specific "about me" without overriding your main bio/about me?
    pub override_description: Option<String>,
    // TODO: override_avatar
    // TODO: override_banner
    /// the roles that this member has
    #[serde(deserialize_with = "deserialize_sorted")]
    pub roles: Vec<RoleId>,

    /// how this member joined the room, moderator only. is None if the origin is unknown.
    // TODO: box
    pub origin: Option<RoomMemberOrigin>,

    /// whether this user is muted by a moderator
    pub mute: bool,

    /// whether this user is deafened by a moderator
    pub deaf: bool,

    /// temporarily prevent a member from communicating
    pub timeout_until: Option<Time>,
    // pub timeout: Option<Timeout>,
    /// whether this user is quarantined by automod
    pub quarantined: bool,
}

/// a room member's timeout
#[record]
#[derive(Default, PartialEq, Eq)]
pub struct Timeout {
    /// when this timeout expires
    pub expires_at: Option<Time>,

    /// moderator only reason for why this timeout exists
    ///
    /// same as the audit log reason
    pub reason: Option<String>,
}

#[record]
#[derive(Default)]
pub struct RoomMemberPut {
    #[schema(required = false, min_length = 1, max_length = 64)]
    #[validate(length(min = 1, max = 64))]
    pub override_name: Option<String>,

    #[schema(required = false, min_length = 1, max_length = 8192)]
    #[validate(length(min = 1, max = 8192))]
    pub override_description: Option<String>,
    // pub override_avatar: Option<String>,
    // maybe flair: Option<String> as a short bit of extra text by the name
    /// whether this user is muted by a moderator
    pub mute: bool,

    /// whether this user is deafened by a moderator
    pub deaf: bool,

    /// the roles that this member has
    #[serde(deserialize_with = "deserialize_sorted")]
    pub roles: Vec<RoleId>,

    /// temporarily prevent a member from communicating
    pub timeout_until: Option<Time>,
}

#[record]
#[derive(Default, Diff)]
#[diff(target = "RoomMember")]
pub struct RoomMemberUpdate {
    #[schema(min_length = 1, max_length = 64)]
    #[validate(length(min = 1, max = 64))]
    #[serde(default, deserialize_with = "some_option")]
    pub override_name: Option<Option<String>>,

    // NOTE: maybe i don't want to let moderators update this?
    // NOTE: it might also be useful to be able to have "shared notes" for
    // moderators, but idk if it should be here or somewhere else
    #[schema(min_length = 1, max_length = 8192)]
    #[validate(length(min = 1, max = 8192))]
    #[serde(default, deserialize_with = "some_option")]
    pub override_description: Option<Option<String>>,

    /// whether this user is muted by a moderator
    pub mute: Option<bool>,

    /// whether this user is deafened by a moderator
    pub deaf: Option<bool>,

    /// the roles that this member has
    #[serde(deserialize_with = "deserialize_sorted_option")]
    pub roles: Option<Vec<RoleId>>,

    /// temporarily prevent a member from communicating
    #[serde(default, deserialize_with = "some_option")]
    pub timeout_until: Option<Option<Time>>,
    // TODO: room member timeout_for
    // /// timeout for server time + this in milliseconds
    // ///
    // /// does nothing if the member is already timed out for longer than this would time them out for
    // ///
    // /// incompatible with timeout_until
    // pub timeout_for: Option<u64>,
}

#[record]
#[derive(PartialEq, Eq)]
#[serde(tag = "type")]
pub enum RoomMemberOrigin {
    /// joined via invite
    Invite {
        /// the invite code they joined with
        code: InviteCode,

        /// the user who created the invite
        inviter: UserId,
    },

    /// this is a bot that was installed
    BotInstall {
        /// the user who installed this bot
        user_id: UserId,
    },

    /// this is a puppet user and was added by a bridge
    Bridged {
        /// the bridge that owns this puppet
        bridge_id: UserId,
    },

    /// this is the room creator
    Creator,

    /// Upgraded from group dm
    GdmUpgrade,

    /// User joined public room directly
    PublicJoin,
}

// TODO: move prune types to a separate module
/// Room member prune
#[record]
pub struct PruneBegin {
    /// include users with these roles
    #[serde(default)]
    pub include_roles: Vec<RoleId>,

    /// prune users inactive for this many days
    #[validate(range(min = 1, max = 90))]
    pub days: u8,

    /// whether to return the number of pruned users in the response
    // endpoint 202 accepted if false, 200 ok if true
    #[serde(default = "bool_true")]
    pub calculate_total: bool,

    /// whether to actually prune or to
    #[serde(default = "bool_true")]
    pub dry_run: bool,
}

/// response for PruneBegin
#[record]
pub struct PruneResponse {
    /// number of pruned users
    pub pruned: u64,
}

#[record]
#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(utoipa::IntoParams))]
pub struct RoomMemberSearch {
    pub query: String,
    #[validate(range(min = 1, max = 100))]
    pub limit: Option<u16>,
}

#[record]
#[derive(Default)]
pub struct RoomMemberSearchAdvanced {
    /// user name, override_name, or id
    #[validate(length(min = 1, max = 64))]
    pub query: Option<String>,

    /// maximum number of results to return
    #[validate(range(min = 1, max = 1024))]
    pub limit: Option<u16>,

    /// has all of these roles
    #[serde(default)]
    pub roles: Vec<RoleId>,

    /// joined from this invite
    pub invite: Option<InviteCode>,

    /// return members who are/aren't timed out
    pub timeout: Option<bool>,

    /// return members who are/aren't room muted
    pub mute: Option<bool>,

    /// return members who are/aren't room deafened
    pub deaf: Option<bool>,

    /// return members who do/don't have a custom nickname
    pub nickname: Option<bool>,

    /// return members who are/aren't server guests
    pub guest: Option<bool>,

    /// members who joined the room before this time
    pub join_before: Option<Time>,

    /// members who joined the room after this time
    pub join_after: Option<Time>,

    /// users who were created before this time
    pub create_before: Option<Time>,

    /// users who were created after this time
    pub create_after: Option<Time>,
}

#[record]
pub struct RoomMemberSearchResponse {
    pub room_members: Vec<RoomMember>,
    pub users: Vec<User>,
}

/// Query parameters for room member delete
#[record]
#[derive(Default)]
#[cfg_attr(feature = "utoipa", derive(utoipa::IntoParams))]
pub struct RoomMemberDeleteQuery {
    pub soft: bool,
}

impl RoomMember {
    pub fn apply_patch(&mut self, patch: RoomMemberUpdate) {
        if let Some(override_name) = patch.override_name {
            self.override_name = override_name;
        }
        if let Some(override_description) = patch.override_description {
            self.override_description = override_description;
        }
        if let Some(mute) = patch.mute {
            self.mute = mute;
        }
        if let Some(deaf) = patch.deaf {
            self.deaf = deaf;
        }
        if let Some(roles) = patch.roles {
            self.roles = roles;
        }
        if let Some(timeout_until) = patch.timeout_until {
            self.timeout_until = timeout_until;
        }
    }

    pub fn apply_put(&mut self, put: RoomMemberPut) {
        self.override_name = put.override_name;
        self.override_description = put.override_description;
        self.mute = put.mute;
        self.deaf = put.deaf;
        self.roles = put.roles;
        self.timeout_until = put.timeout_until;
    }
}

// TEMP: compat
pub use crate::v1::types::room_ban::*;
