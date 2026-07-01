//! identifiers used in the document system

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::ids::{DocumentBranchId, DocumentTagId};

/// an exact version of a document at a point in time
///
/// serialized as `branch-uuid@seq`
// TODO: document how this is like/unlike yrs/yjs' `StateVector`
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(try_from = "String", into = "String")
)]
pub struct DocumentRevisionId {
    pub branch_id: DocumentBranchId,
    pub seq: u64,
}

/// a resolvable reference to a `DocumentRevisionId`
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(try_from = "String", into = "String")
)]
// TODO: use tuple variant syntax instead of struct variant syntax
pub enum DocumentRevisionRef {
    /// the current head (latest revision) of a branch
    ///
    /// serialized as `branch-id`
    Branch { branch_id: DocumentBranchId },

    /// a specific revision
    ///
    /// serialized as `branch-id@seq`
    Revision { version_id: DocumentRevisionId },

    /// the revision pointed to by this tag
    ///
    /// serialized as `~tag`
    Tag { tag_id: DocumentTagId },
}

// ===== DocumentRevisionId impls =====

impl std::fmt::Display for DocumentRevisionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.branch_id, self.seq)
    }
}

impl std::str::FromStr for DocumentRevisionId {
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

// ===== DocumentRevisionRef impls =====

impl std::fmt::Display for DocumentRevisionRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Branch { branch_id } => write!(f, "{}", branch_id),
            Self::Revision { version_id } => write!(f, "{}", version_id),
            Self::Tag { tag_id } => write!(f, "~{}", tag_id),
        }
    }
}

impl std::str::FromStr for DocumentRevisionRef {
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

impl From<DocumentRevisionRef> for String {
    fn from(id: DocumentRevisionRef) -> Self {
        id.to_string()
    }
}

impl TryFrom<String> for DocumentRevisionRef {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

#[cfg(feature = "utoipa")]
mod document_revision_id_schema {
    use utoipa::{PartialSchema, ToSchema, openapi::ObjectBuilder};

    use super::DocumentRevisionRef;

    impl ToSchema for DocumentRevisionRef {}

    impl PartialSchema for DocumentRevisionRef {
        fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
            ObjectBuilder::new()
                .schema_type(utoipa::openapi::schema::Type::String)
                .description(Some(
                    "A revision of a document at a point in time.\n\n\
                    Serialized as:\n\
                    - `branch-id` for the current head of a branch\n\
                    - `branch-uuid@seq` for a specific revision\n\
                    - `~tag` for a specific tag",
                ))
                .build()
                .into()
        }
    }
}
