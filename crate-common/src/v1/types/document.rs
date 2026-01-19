#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{
    ids::{DocumentBranchId, DocumentTagId},
    misc::Time,
    util::some_option,
    ChannelId, RoomMember, ThreadMember, User, UserId,
};

/// info about a document
// NOTE: this will probably be included in Channel as `document: Option<Document>`
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
pub struct DocumentBranch {
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
    pub state: DocumentBranchState,

    /// the parent branch that this branch was forked from
    ///
    /// is None if this is the default branch
    pub parent_branch_id: Option<DocumentBranchId>,
    // pub parent_commit_id: Option<DocumentCommitId>,
    // pub merged_at: Option<Time>,
    // pub merged_into: Option<DocumentBranchId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum DocumentBranchState {
    /// currently being edited
    Active,

    /// this branch was closed without being merged
    ///
    /// this branch no longer shows up in any listings
    Closed,

    /// this has been merged into a document
    ///
    /// this branch no longer shows up in any listings
    Merged,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct DocumentBranchCreate {
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
pub struct DocumentBranchPatch {
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
pub struct DocumentBranchMerge {
    // /// overwrite where to merge to, defaults to the parent branch
    // pub target_branch_id: Option<DocumentBranchId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct DocumentBranchMergeResult {
    pub status: DocumentBranchMergeResultStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum DocumentBranchMergeResultStatus {
    /// no existing changes! the cleanest merge
    FastForward,

    /// cleanly merged with existing changes
    Merged,

    /// can't merge because there are too many conflicts
    Conflicted,
}

/// a revision of a document at a point in time
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum DocumentRevisionId {
    /// the current head of this branch
    ///
    /// serialized as `branch-id`
    Branch { branch_id: DocumentBranchId },

    /// this one specific revision
    ///
    /// serialized as `branch-uuid@seq`
    Revision {
        branch_id: DocumentBranchId,
        seq: u64,
    },

    /// this one specific revision
    ///
    /// serialized as `~tag`
    Tag { tag_id: DocumentTagId },
}

#[cfg(feature = "serde")]
impl Serialize for DocumentRevisionId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            DocumentRevisionId::Branch { branch_id } => {
                serializer.serialize_str(&branch_id.to_string())
            }
            DocumentRevisionId::Revision { branch_id, seq } => {
                serializer.serialize_str(&format!("{}@{}", branch_id, seq))
            }
            DocumentRevisionId::Tag { tag_id } => serializer.serialize_str(&format!("~{}", tag_id)),
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for DocumentRevisionId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if let Some(tag_str) = s.strip_prefix('~') {
            let tag_id = tag_str.parse().map_err(serde::de::Error::custom)?;
            Ok(DocumentRevisionId::Tag { tag_id })
        } else if let Some((branch_str, seq_str)) = s.split_once('@') {
            let branch_id = branch_str.parse().map_err(serde::de::Error::custom)?;
            let seq = seq_str.parse().map_err(serde::de::Error::custom)?;
            Ok(DocumentRevisionId::Revision { branch_id, seq })
        } else {
            let branch_id = s.parse().map_err(serde::de::Error::custom)?;
            Ok(DocumentRevisionId::Branch { branch_id })
        }
    }
}

/// a named version
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct DocumentTag {
    /// the unique identifier for this tag
    pub id: DocumentTagId,

    /// when this tag was created
    pub created_at: Time,

    /// when this tag was last updated
    pub updated_at: Time,

    /// who created this tag
    ///
    /// may be None if the creator doesnt exist
    pub creator_id: Option<UserId>,

    pub branch_id: DocumentBranchId,
    pub revision_seq: u64,

    /// one line description
    #[cfg_attr(feature = "utoipa", schema(max_length = 128))]
    #[cfg_attr(feature = "validator", validate(length(max = 128)))]
    pub summary: String,

    /// optional more detailed description
    #[cfg_attr(feature = "utoipa", schema(required = false, max_length = 4096))]
    #[cfg_attr(feature = "validator", validate(length(max = 4096)))]
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct DocumentTagCreate {
    /// one line description
    #[cfg_attr(feature = "utoipa", schema(max_length = 128))]
    #[cfg_attr(feature = "validator", validate(length(max = 128)))]
    pub summary: String,

    /// optional more detailed description
    #[cfg_attr(feature = "utoipa", schema(required = false, max_length = 4096))]
    #[cfg_attr(feature = "validator", validate(length(max = 4096)))]
    pub description: Option<String>,

    pub revision: DocumentRevisionId,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct DocumentTagPatch {
    /// one line description
    #[cfg_attr(feature = "utoipa", schema(required = false, max_length = 128))]
    #[cfg_attr(feature = "validator", validate(length(max = 128)))]
    pub summary: Option<String>,

    /// optional more detailed description
    #[cfg_attr(feature = "utoipa", schema(required = false, max_length = 4096))]
    #[cfg_attr(feature = "validator", validate(length(max = 4096)))]
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub description: Option<Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(IntoParams))]
pub struct HistoryParams {
    /// split group whenever author changes
    pub by_author: Option<bool>,

    /// split group whenever a tag is created
    pub by_tag: Option<bool>,

    /// every n seconds
    pub by_time: Option<u32>,

    /// every n changes
    pub by_changes: Option<u32>,

    /// continue listing history from here
    pub cursor: Option<String>,

    /// the maximum number of items to return.
    // FIXME: default 10, max 1024
    pub limit: Option<u16>,
}

/// a set of changes made to a document
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Changeset {
    /// the created_at time of the first change
    pub start_time: Time,

    /// the created_at time of the last change
    pub end_time: Time,

    /// every author that contributed to this change group
    pub authors: Vec<UserId>,

    /// number of graphemes added
    pub stat_added: u64,

    /// number of graphemes removed
    pub stat_removed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct HistoryPagination {
    /// the resulting changesets, ordered oldest to newest
    pub changesets: Vec<Changeset>,

    /// a user object for every referenced user_id
    pub users: Vec<User>,

    /// a room member object for every referenced user_id
    pub room_members: Vec<RoomMember>,

    /// a thread member object for every referenced user_id
    pub thread_members: Vec<ThreadMember>,
}
