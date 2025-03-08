use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use super::{RoleId, RoomId, UserId};

use crate::v1::types::util::{some_option, Diff, Time};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct RoomMember {
    pub user_id: UserId,
    pub room_id: RoomId,

    #[cfg_attr(feature = "validator", validate(nested))]
    #[serde(flatten)]
    pub membership: RoomMembership,

    /// When this member's membership last changed (joined, left, was kicked, or banned).
    pub membership_updated_at: Time,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[serde(tag = "membership")]
pub enum RoomMembership {
    /// joined
    Join {
        override_name: Option<String>,
        override_description: Option<String>,
        // override_avatar: z.string().url().or(z.literal("")),
        roles: Vec<RoleId>,
        // muted_until: Option<Time>,
        // /// how this member joined the room
        // #[serde(flatten)]
        // origin: RoomMemberOrigin,
    },

    /// kicked or left, can rejoin with an invite. todo: can still view messages up until then
    Leave {
        // TODO: keep roles on leave?
        // TODO: copy kick/ban reason here
        // /// user supplied reason why this user was banned
        // reason: Option<String>,
        // /// which user caused the kick, or None if the user left themselves
        // user_id: Option<UserId>,
    },

    /// banned. todo: can still view messages up until they were banned
    Ban {
        // /// user supplied reason why this user was banned
        // reason: Option<String>,
        // /// which user caused the ban
        // user_id: Option<UserId>,
        // banned_until: Option<Time>,
    },
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
        match &other.membership {
            RoomMembership::Join {
                override_name,
                override_description,
                roles: _,
            } => {
                self.override_name.changes(override_name)
                    || self.override_description.changes(override_description)
            }
            _ => false,
        }
    }
}

#[cfg(feature = "validator")]
mod val {
    use super::RoomMembership;
    use serde_json::json;
    use validator::{Validate, ValidateLength, ValidationError, ValidationErrors};

    impl Validate for RoomMembership {
        fn validate(&self) -> Result<(), ValidationErrors> {
            let mut v = ValidationErrors::new();
            match self {
                RoomMembership::Join {
                    override_name,
                    override_description,
                    roles: _,
                } => {
                    if override_name
                        .as_ref()
                        .is_some_and(|n| n.validate_length(Some(1), Some(64), None))
                    {
                        let mut err = ValidationError::new("length");
                        err.add_param("max".into(), &json!(64));
                        err.add_param("min".into(), &json!(1));
                        v.add("override_name", err);
                    }
                    if override_description
                        .as_ref()
                        .is_some_and(|n| n.validate_length(Some(1), Some(8192), None))
                    {
                        let mut err = ValidationError::new("length");
                        err.add_param("max".into(), &json!(8192));
                        err.add_param("min".into(), &json!(1));
                        v.add("override_description", err);
                    }
                }
                RoomMembership::Leave {} => {}
                RoomMembership::Ban {} => {}
            }
            if v.is_empty() {
                Ok(())
            } else {
                Err(v)
            }
        }
    }
}
