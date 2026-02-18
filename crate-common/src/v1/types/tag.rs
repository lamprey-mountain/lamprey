#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{misc::Color, ChannelId, TagId};

#[cfg(feature = "serde")]
use crate::v1::types::util::{default_false_opt, some_option};

/// a tag that can be applied to a thread
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Tag {
    pub id: TagId,

    pub channel_id: ChannelId,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: String,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 8192))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub description: Option<String>,

    /// the color of this tag
    #[cfg_attr(feature = "utoipa", schema(required = false))]
    pub color: Option<Color>,

    /// whether this tag is archived. this tag cant be applied to any new threads and won't appear in the tag picker.
    pub archived: bool,

    /// only members with ThreadEdit or ThreadManage can apply this tag
    pub restricted: bool,

    /// total number of threads with this tag (excluding archived threads)
    pub active_thread_count: u64,

    /// total number of threads with this tag (including archived threads)
    pub total_thread_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct TagCreate {
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: String,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 8192))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub description: Option<String>,

    pub color: Option<Color>,

    #[cfg_attr(feature = "serde", serde(default))]
    pub restricted: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct TagPatch {
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 64)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: Option<String>,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 8192))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub description: Option<Option<String>>,

    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub color: Option<Option<Color>>,

    pub archived: Option<bool>,
    pub restricted: Option<bool>,
}

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct TagDeleteQuery {
    #[cfg_attr(feature = "serde", serde(default))]
    pub force: bool,
}

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct TagSearchQuery {
    pub query: String,

    /// deny, allow, require tag to be archived
    ///
    /// default: deny
    #[cfg_attr(feature = "serde", serde(default = "default_false_opt"))]
    pub archived: Option<bool>,
}

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct TagListQuery {
    /// deny, allow, require tag to be archived
    ///
    /// default: deny
    #[cfg_attr(feature = "serde", serde(default = "default_false_opt"))]
    pub archived: Option<bool>,
}
