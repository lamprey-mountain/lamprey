#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{
    ids::{DocumentBranchId, DocumentTagId},
    misc::Time,
    util::{some_option, Diff},
    ChannelId, RoomMember, ThreadMember, User, UserId,
};

pub mod crdt;
pub mod serialized;

pub use crdt::{DocumentStateVector, DocumentUpdate};

/// channel metadata for a document
///
/// these properties only exist for documents in wiki channels
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

    /// custom url path to put this at
    #[cfg_attr(feature = "utoipa", schema(required = false, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(max = 64)))]
    pub slug: Option<String>,

    /// if this document has been published
    pub published: Option<DocumentPublished>,
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

    /// the parent version that this branch was forked from
    ///
    /// is None if this is the default branch
    pub parent_id: Option<DocumentVersionId>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
pub struct DocumentBranchListParams {
    /// only include branches with these states
    ///
    /// defaults to only Active
    #[serde(default)]
    pub state: Vec<DocumentBranchState>,
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

// NOTE: not useful; may be removed later?
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct DocumentBranchMergeResult {
    pub status: DocumentBranchMergeResultStatus,
}

// NOTE: not useful; may be removed later?
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

/// a version of a document at a point in time
///
/// serialized as `branch-uuid@seq`
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "String", into = "String"))]
pub struct DocumentVersionId {
    pub branch_id: DocumentBranchId,
    pub seq: u64,
}

impl std::fmt::Display for DocumentVersionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.branch_id, self.seq)
    }
}

impl std::str::FromStr for DocumentVersionId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (branch_str, seq_str) = s
            .split_once('@')
            .ok_or_else(|| "invalid format".to_string())?;
        let branch_id = branch_str.parse().map_err(|e: uuid::Error| e.to_string())?;
        let seq = seq_str
            .parse()
            .map_err(|e: std::num::ParseIntError| e.to_string())?;
        Ok(Self { branch_id, seq })
    }
}

impl From<DocumentVersionId> for String {
    fn from(id: DocumentVersionId) -> Self {
        id.to_string()
    }
}

impl TryFrom<String> for DocumentVersionId {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

/// a revision of a document at a point in time
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "String", into = "String"))]
pub enum DocumentRevisionId {
    /// the current head of this branch
    ///
    /// serialized as `branch-id`
    Branch { branch_id: DocumentBranchId },

    /// this one specific revision
    ///
    /// serialized as `branch-uuid@seq`
    Revision { version_id: DocumentVersionId },

    /// this one specific revision
    ///
    /// serialized as `~tag`
    Tag { tag_id: DocumentTagId },
}

impl std::fmt::Display for DocumentRevisionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Branch { branch_id } => write!(f, "{}", branch_id),
            Self::Revision { version_id } => write!(f, "{}", version_id),
            Self::Tag { tag_id } => write!(f, "~{}", tag_id),
        }
    }
}

impl std::str::FromStr for DocumentRevisionId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(tag_str) = s.strip_prefix('~') {
            let tag_id = tag_str.parse().map_err(|e: uuid::Error| e.to_string())?;
            Ok(Self::Tag { tag_id })
        } else if s.contains('@') {
            let version_id = s.parse()?;
            Ok(Self::Revision { version_id })
        } else {
            let branch_id = s.parse().map_err(|e: uuid::Error| e.to_string())?;
            Ok(Self::Branch { branch_id })
        }
    }
}

impl From<DocumentRevisionId> for String {
    fn from(id: DocumentRevisionId) -> Self {
        id.to_string()
    }
}

impl TryFrom<String> for DocumentRevisionId {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
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
#[cfg_attr(feature = "utoipa", derive(IntoParams, ToSchema))]
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
    // use a string for opaqueness, but maybe make this a DocumentVersionId internally
    pub cursor: Option<String>,

    /// the maximum number of items to return.
    // FIXME: default 10, max 1024
    pub limit: Option<u16>,
    // TODO: filtering
    // pub before_time: Option<Time>,
    // pub before_revision: Option<DocumentRevisionId>,
    // pub after_time: Option<Time>,
    // pub after_revision: Option<DocumentRevisionId>,
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

    /// the document this changeset applies to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_id: Option<ChannelId>,
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

    /// document tags that are part of the range
    pub document_tags: Vec<DocumentTag>,
}

/// parameters for getting a crdt
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(IntoParams))]
pub struct DocumentCrdtDiffParams {
    pub sv: Option<DocumentStateVector>,
}

/// parameters for updating a crdt
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct DocumentCrdtApply {
    pub update: DocumentUpdate,
}

/// channel metadata for a wiki
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Wiki {
    /// whether to allow indexing by search engines
    #[cfg_attr(feature = "serde", serde(default))]
    pub allow_indexing: bool,

    /// the id of the document that should be used as the main/home/index page
    pub page_index: Option<ChannelId>,

    /// the id of the document that should be used as the 404/not found page
    pub page_notfound: Option<ChannelId>,
}

/// info about when a document was published
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct DocumentPublished {
    /// when this document was published
    pub time: Time,

    /// the revision of the document that was published
    pub revision: DocumentRevisionId,

    /// published but doesnt show up in any search results
    #[cfg_attr(feature = "serde", serde(default))]
    pub unlisted: bool,
}

/// update a serdoc
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SerdocPut {
    pub root: serialized::SerdocRoot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct DocumentPatch {
    pub draft: Option<bool>,
    pub template: Option<bool>,
    #[serde(default, deserialize_with = "some_option")]
    pub archived: Option<Option<DocumentArchivedPatch>>,
    pub slug: Option<Option<String>>,
    #[serde(default, deserialize_with = "some_option")]
    pub published: Option<Option<DocumentPublishedPatch>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct DocumentPublishedPatch {
    pub revision: Option<DocumentRevisionId>,
    #[serde(default)]
    pub unlisted: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct DocumentArchivedPatch {
    #[serde(default, deserialize_with = "some_option")]
    pub reason: Option<Option<String>>,
}

impl Diff<DocumentArchived> for DocumentArchivedPatch {
    fn changes(&self, other: &DocumentArchived) -> bool {
        self.reason.changes(&other.reason)
    }
}

impl Diff<DocumentPublished> for DocumentPublishedPatch {
    fn changes(&self, other: &DocumentPublished) -> bool {
        self.revision.changes(&other.revision) || self.unlisted.changes(&other.unlisted)
    }
}

impl Diff<Document> for DocumentPatch {
    fn changes(&self, other: &Document) -> bool {
        self.draft.changes(&other.draft)
            || self.template.changes(&other.template)
            || self.slug.changes(&other.slug)
            // TODO: figure out if i can simplify this
            || (match (&self.archived, &other.archived) {
                (None, _) => false,
                (Some(None), None) => false,
                (Some(None), Some(_)) => true,
                (Some(Some(a)), Some(b)) => a.changes(b),
                (Some(Some(_)), None) => true,
            })
            || (match (&self.published, &other.published) {
                (None, _) => false,
                (Some(None), None) => false,
                (Some(None), Some(_)) => true,
                (Some(Some(a)), Some(b)) => a.changes(b),
                (Some(Some(_)), None) => true,
            })
    }
}

impl Diff<Wiki> for WikiPatch {
    fn changes(&self, other: &Wiki) -> bool {
        self.allow_indexing.changes(&other.allow_indexing)
            || self.page_index.changes(&other.page_index)
            || self.page_notfound.changes(&other.page_notfound)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct WikiPatch {
    pub allow_indexing: Option<bool>,

    #[serde(default, deserialize_with = "some_option")]
    pub page_index: Option<Option<ChannelId>>,

    #[serde(default, deserialize_with = "some_option")]
    pub page_notfound: Option<Option<ChannelId>>,
}
