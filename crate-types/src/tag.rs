use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::{Color, RoleId, RoleVerId, RoomId, TagId};

/// a tag that can be applied to things
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Tag {
    pub id: TagId,
    pub version_id: RoleVerId,

    // maybe make tags separate from rooms?
    // pub room_id: Option<RoomId>,
    pub room_id: RoomId,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: String,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 8192))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub description: Option<String>,

    /// the color of this tag
    pub color: Color,

    /// whether this tag is exclusive. functions similarly to forgejo
    pub is_exclusive: bool,

    /// whether this tag is archived. cant be applied to any new threads or appear in pickers but still exists.
    pub is_archived: bool,

    /// restrict who can apply this tag. default: everyone
    pub restrict: Option<Vec<RoleId>>,

    /// if this tag includes other tags (composition). ie. tag `fruits` might include `apples` and `oranges`
    pub includes: Option<Vec<TagId>>,
}
