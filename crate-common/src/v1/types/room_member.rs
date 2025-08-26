use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use super::{RoleId, RoomId, UserId};

use crate::v1::types::{
    util::{some_option, Diff, Time},
    User,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct RoomMember {
    // in the future, i might want to get rid of user_id and generally scope user profiles to rooms
    // pub member_id: MemberId,
    pub user_id: UserId,
    pub room_id: RoomId,

    pub membership: RoomMembership,

    /// When this member's membership last changed (joined, left, was kicked, or banned).
    #[deprecated]
    pub membership_updated_at: Time,

    /// When this member joined the room
    pub joined_at: Time,
    // TODO?: pub left_at: Option<Time>,
    /// aka nickname
    // TODO: rename to `nick`?
    pub override_name: Option<String>,

    /// like nickname, but for your description/bio/about
    // TODO: remove. maybe replace with a room-specific "about me" without overriding your main bio/about me?
    pub override_description: Option<String>,

    // TODO: per-room avatars? override_avatar: z.string().url().or(z.literal("")),
    /// the roles that this member has
    pub roles: Vec<RoleId>,
    // muted_until: Option<Time>, // timeouts
    // /// how this member joined the room
    // // should be moderator only
    // #[serde(flatten)]
    // origin: RoomMemberOrigin,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum RoomMembership {
    /// joined
    Join,

    /// left, can rejoin with an invite. todo: can still view messages up until then
    Leave,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoomBan {
    pub user: User,
    pub reason: Option<String>,
    pub created_at: Option<Time>,
    pub expires_at: Option<Time>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoomBanCreate {
    pub expires_at: Option<Time>,
}

// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// #[serde(tag = "origin")]
// pub enum RoomMemberOrigin {
//     /// joined via invite
//     Invite { origin_code: InviteCode },

//     /// joined via invite which is now expired
//     InviteExpired { origin_code: InviteCode },

//     /// added by another user (puppet)
//     Added { origin_user_id: UserId },
// }

impl Diff<RoomMember> for RoomMemberPatch {
    fn changes(&self, other: &RoomMember) -> bool {
        self.override_name.changes(&other.override_name)
            || self
                .override_description
                .changes(&other.override_description)
    }
}
