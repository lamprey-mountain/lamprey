#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{ids::DocumentBranchId, misc::Time, ChannelId, UserId};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Document {
    /// whether this document is a draft
    ///
    /// drafts aren't shown publicly, and can only be seen by the user who created it.
    pub draft: bool,

    /// populated if this is archived
    ///
    /// hide this document in listings by default, and show a warning/notice banner on top.
    pub archived: Option<DocumentArchived>,

    /// whether this document is a reusable template
    pub template: bool,
}

/// info about an archived document
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct DocumentArchived {
    pub archived_at: Time,
    pub reason: Option<String>,
}

/// a lightweight alternate editing context for a document
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Branch {
    pub id: DocumentBranchId,
    pub document_id: ChannelId,

    /// the user who created this branch
    pub creator_id: UserId,

    /// the name of this branch
    #[cfg_attr(feature = "utoipa", schema(required = false, max_length = 256))]
    pub name: Option<String>,

    /// when this branch was created
    pub created_at: Time,

    /// Whether this is the default branch.
    ///
    /// The default branch cannot be deleted and has the same id as the document
    // NOTE: maybe i want to rename this to "main" or "master" or something?
    pub default: bool,

    /// Whether this is a private branch.
    ///
    /// Private branches are only visible to the user who created this branch, similar to draft documents.
    pub private: bool,

    /// the current state of this branch
    pub state: BranchState,

    /// the parent branch that this branch was forked from
    ///
    /// is None if this is the default branch
    pub parent_branch_id: Option<DocumentBranchId>,
    // pub parent_commit_id: Option<DocumentCommitId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum BranchState {
    /// currently being edited
    Open,

    /// this branch was closed without being merged
    ///
    /// this branch no longer shows up in any listings
    Canceled,

    /// this has been merged into a document
    ///
    /// this branch no longer shows up in any listings
    Merged,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct BranchCreate {
    #[cfg_attr(feature = "utoipa", schema(required = false, max_length = 256))]
    #[cfg_attr(feature = "validator", validate(length(max = 256)))]
    pub name: Option<String>,

    #[cfg_attr(feature = "serde", serde(default))]
    pub private: bool,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct BranchPatch {
    #[cfg_attr(feature = "utoipa", schema(required = false, max_length = 256))]
    #[cfg_attr(feature = "validator", validate(length(max = 256)))]
    pub name: Option<Option<String>>,

    /// once public, branches cannot be made private again
    #[cfg_attr(feature = "serde", serde(default))]
    pub private: bool,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct BranchMerge {
    // /// overwrite where to merge to, defaults to the parent branch
    // pub target_branch_id: Option<DocumentBranchId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct BranchMergeResult {
    pub status: BranchMergeResultStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum BranchMergeResultStatus {
    /// no existing changes! the cleanest merge
    FastForward,

    /// cleanly merged with existing changes
    Merged,

    /// can't merge because there are too many conflicts
    Conflicted,
}
