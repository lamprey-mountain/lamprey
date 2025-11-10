#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use super::{RoleId, RoomId, User, UserId};

use crate::v1::types::{
    util::{some_option, Diff, Time},
    InviteCode,
};

fn bool_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct RoomMember {
    pub user_id: UserId,
    pub room_id: RoomId,

    // NOTE: this will always be Join
    pub membership: RoomMembership,

    /// When this member joined the room
    pub joined_at: Time,

    /// aka nickname
    // TODO: rename to `nick`
    pub override_name: Option<String>,

    /// like nickname, but for your description/bio/about
    // TODO: remove. maybe replace with a room-specific "about me" without overriding your main bio/about me?
    pub override_description: Option<String>,

    /// the roles that this member has
    pub roles: Vec<RoleId>,

    /// how this member joined the room, moderator only. is None if the origin is unknown.
    pub origin: Option<RoomMemberOrigin>,

    /// whether this user is muted by a moderator
    pub mute: bool,

    /// whether this user is deafened by a moderator
    pub deaf: bool,

    /// temporarily prevent a member from communicating
    pub timeout_until: Option<Time>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct RoomMemberPut {
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 64)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub override_name: Option<String>,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 8192)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub override_description: Option<String>,
    // pub override_avatar: Option<String>,
    // maybe flair: Option<String> as a short bit of extra text by the name
    /// whether this user is muted by a moderator
    pub mute: Option<bool>,

    /// whether this user is deafened by a moderator
    pub deaf: Option<bool>,

    /// the roles that this member has
    pub roles: Option<Vec<RoleId>>,

    /// temporarily prevent a member from communicating
    pub timeout_until: Option<Time>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct RoomMemberPatch {
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    #[serde(default, deserialize_with = "some_option")]
    pub override_name: Option<Option<String>>,

    // NOTE: maybe i don't want to let moderators update this?
    // NOTE: it might also be useful to be able to have "shared notes" for
    // moderators, but idk if it should be here or somewhere else
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 8192))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    #[serde(default, deserialize_with = "some_option")]
    pub override_description: Option<Option<String>>,
    // #[serde(default, deserialize_with = "some_option")]
    // pub override_avatar: Option<Option<String>>,
    /// whether this user is muted by a moderator
    pub mute: Option<bool>,

    /// whether this user is deafened by a moderator
    pub deaf: Option<bool>,

    /// the roles that this member has
    pub roles: Option<Vec<RoleId>>,

    /// temporarily prevent a member from communicating
    #[serde(default, deserialize_with = "some_option")]
    pub timeout_until: Option<Option<Time>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum RoomMembership {
    /// joined
    Join,

    /// left, can rejoin with an invite. todo: can still view messages up until then
    Leave,
}

/// represents a restriction on who can join the room
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoomBan {
    /// the user who is banned
    pub user_id: UserId,

    /// the supplied reason why this user should be banned
    pub reason: Option<String>,

    /// when the ban was created
    pub created_at: Time,

    /// when the ban expires
    pub expires_at: Option<Time>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoomBanCreate {
    pub expires_at: Option<Time>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
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

// in the future, there will be multiple types of bans. right now there are just user bans.
// BanId would be changed from UserId to another uuid newtype
// pub enum RoomBanType {
//     User {
//         /// the user who is banned
//         user_id: UserId,
//     },

//     Ip {
//         /// the ip address(es) which are banned
//         cidr: IpCidr,
//     },

//     // for when federation is implemented
//     Server {
//         /// the host who is banned
//         host: String,
//     },
// }

/// create many bans at once
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct RoomBanBulkCreate {
    /// who to ban
    #[serde(default)]
    #[validate(length(min = 1, max = 256))]
    pub target_ids: Vec<UserId>,

    /// when the ban expires
    pub expires_at: Option<Time>,
}

/// Room member prune
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct PruneBegin {
    /// include users with these roles
    #[serde(default)]
    pub include_roles: Vec<RoleId>,

    /// prune users inactive for this many days
    #[cfg_attr(feature = "validator", validate(range(min = 1, max = 90)))]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PruneResponse {
    /// number of pruned users
    pub pruned: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::IntoParams, ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct RoomMemberSearch {
    pub query: String,
    #[cfg_attr(feature = "validator", validate(range(min = 1, max = 100)))]
    pub limit: Option<u16>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct RoomMemberSearchAdvanced {
    /// user name, override_name, or id
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub query: Option<String>,

    /// maximum number of results to return
    #[cfg_attr(feature = "validator", validate(range(min = 1, max = 1024)))]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoomMemberSearchResponse {
    pub room_members: Vec<RoomMember>,
    pub users: Vec<User>,
}

impl Diff<RoomMember> for RoomMemberPatch {
    fn changes(&self, other: &RoomMember) -> bool {
        self.override_name.changes(&other.override_name)
            || self
                .override_description
                .changes(&other.override_description)
            || self.mute.changes(&other.mute)
            || self.deaf.changes(&other.deaf)
            || self.roles.changes(&other.roles)
            || self.timeout_until.changes(&other.timeout_until)
    }
}
