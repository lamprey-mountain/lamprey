use lamprey_macros::record;

use crate::v1::types::{ChannelId, TagId, misc::Color};

#[cfg(feature = "serde")]
use crate::v1::types::util::{default_false_opt, some_option};

/// a tag that can be applied to a thread
// TODO: rename to ThreadTag or ForumTag
#[record]
#[derive(PartialEq, Eq)]
pub struct Tag {
    pub id: TagId,

    // TODO: remove?
    pub channel_id: ChannelId,

    #[schema(min_length = 1, max_length = 64)]
    #[validate(length(min = 1, max = 64))]
    pub name: String,

    #[schema(min_length = 1, max_length = 8192)]
    #[validate(length(min = 1, max = 8192))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// the color of this tag
    #[schema(required = false)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,

    /// whether this tag is archived
    ///
    /// archived tags cant be applied to any new threads and won't appear in the tag picker.
    pub archived: bool,

    /// only members with ThreadEdit or ThreadManage can apply this tag
    pub restricted: bool,

    /// total number of threads with this tag (excluding archived threads)
    pub active_thread_count: u64,

    /// total number of threads with this tag (including archived threads)
    pub total_thread_count: u64,

    /// if this tag should be considered a spoiler
    pub spoiler: bool,
}

/// minimal data needed to render a tag
#[record]
#[derive(PartialEq, Eq)]
pub struct TagMinimal {
    pub id: TagId,

    #[schema(min_length = 1, max_length = 64)]
    #[validate(length(min = 1, max = 64))]
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,

    pub spoiler: bool,
}

#[record]
#[derive(PartialEq, Eq)]
pub struct TagCreate {
    #[schema(min_length = 1, max_length = 64)]
    #[validate(length(min = 1, max = 64))]
    pub name: String,

    #[schema(min_length = 1, max_length = 8192)]
    #[validate(length(min = 1, max = 8192))]
    pub description: Option<String>,

    pub color: Option<Color>,

    #[serde(default)]
    pub restricted: bool,

    #[serde(default)]
    pub spoiler: bool,
}

#[record]
#[derive(Default, PartialEq, Eq)]
pub struct TagPatch {
    #[schema(required = false, min_length = 1, max_length = 64)]
    #[validate(length(min = 1, max = 64))]
    pub name: Option<String>,

    #[schema(min_length = 1, max_length = 8192)]
    #[validate(length(min = 1, max = 8192))]
    #[serde(default, deserialize_with = "some_option")]
    pub description: Option<Option<String>>,

    #[serde(default, deserialize_with = "some_option")]
    pub color: Option<Option<Color>>,

    pub archived: Option<bool>,
    pub restricted: Option<bool>,
    pub spoiler: Option<bool>,
}

#[record]
#[derive(Default)]
#[cfg_attr(feature = "utoipa", derive(utoipa::IntoParams))]
pub struct TagDeleteQuery {
    #[serde(default)]
    pub force: bool,
}

#[record]
#[derive(Default)]
#[cfg_attr(feature = "utoipa", derive(utoipa::IntoParams))]
pub struct TagSearchQuery {
    pub query: String,

    /// deny, allow, require tag to be archived
    ///
    /// default: deny
    #[serde(default = "default_false_opt")]
    pub archived: Option<bool>,
}

#[record]
#[derive(Default)]
#[cfg_attr(feature = "utoipa", derive(utoipa::IntoParams))]
pub struct TagListQuery {
    /// deny, allow, require tag to be archived
    ///
    /// default: deny
    #[serde(default = "default_false_opt")]
    pub archived: Option<bool>,
}
