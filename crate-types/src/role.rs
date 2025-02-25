use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::util::{
    deserialize_sorted_permissions, deserialize_sorted_permissions_option, some_option, Diff,
};

use super::{Permission, RoleId, RoleVerId, RoomId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Role {
    pub id: RoleId,
    pub version_id: RoleVerId,
    pub room_id: RoomId,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: String,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 8192))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub description: Option<String>,

    #[serde(deserialize_with = "deserialize_sorted_permissions")]
    pub permissions: Vec<Permission>,

    pub is_self_applicable: bool,
    pub is_mentionable: bool,
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct RoleCreateRequest {
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: String,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 8192)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    #[serde(default)]
    pub description: Option<String>,

    #[serde(default)]
    pub permissions: Vec<Permission>,

    #[serde(default)]
    pub is_self_applicable: bool,

    /// if this role can be mentioned by members
    #[serde(default)]
    pub is_mentionable: bool,

    // this might be better..?
    // pub restrict_mentions: Option<Vec<RoleId | UserId>>,
    /// if this role is applied by default to all new members
    #[serde(default)]
    pub is_default: bool,
    // the main reason this doesn't exist yet is because i've seen in
    // discord how the ui can become extremely unreadable, cluttered, and
    // in general color vomit. plus there's the whole "illegable contrast
    // in light/dark mode" thing.
    //
    // i also don't really like the psychological effects of colored names,
    // since i've seen people act differently when someone with a differently
    // colored name shows up (eg. moderators)
    //
    // still, it can be very useful. i'm not sure what's the best way to
    // implement this though; definitely not copying discord here.
    //
    // pub color: Color,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct RolePatch {
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 64)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: Option<String>,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 8192))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    #[serde(default, deserialize_with = "some_option")]
    pub description: Option<Option<String>>,

    #[serde(default, deserialize_with = "deserialize_sorted_permissions_option")]
    pub permissions: Option<Vec<Permission>>,

    pub is_self_applicable: Option<bool>,
    pub is_mentionable: Option<bool>,
    pub is_default: Option<bool>,
}

impl Diff<Role> for RolePatch {
    fn changes(&self, other: &Role) -> bool {
        self.name.changes(&other.name)
            || self.description.changes(&other.description)
            || self.is_self_applicable.changes(&other.is_self_applicable)
            || self.is_mentionable.changes(&other.is_mentionable)
            || self.is_default.changes(&other.is_default)
            || self.permissions.changes(&other.permissions)
    }
}
