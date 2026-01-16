use std::fmt::{self, Display};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

use crate::v1::types::{util::Time, RoomMember, User, UserId};

pub trait PaginationKey: Display + Clone + PartialEq + Eq + PartialOrd + Ord {
    fn min() -> Self;
    fn max() -> Self;
}

// TODO: use strings instead of PaginationKey?
#[derive(Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PaginationQuery<K: PaginationKey> {
    /// The key to start paginating from. Not inclusive. Optional.
    pub from: Option<K>,

    /// The key to stop paginating at. Not inclusive. Optional.
    pub to: Option<K>,

    /// Whether to paginate forwards or backwards. Defaults to forwards. Paginating backwards does not reverse the order of items.
    pub dir: Option<PaginationDirection>,

    /// The maximum number of items to return.
    pub limit: Option<u16>,
}

// unfortunately, utoipa has issues with nested generics (Option<I>)
// TODO: better documentation generation
#[cfg(feature = "utoipa")]
impl<K: PaginationKey> IntoParams for PaginationQuery<K> {
    fn into_params(
        parameter_in_provider: impl Fn() -> Option<utoipa::openapi::path::ParameterIn>,
    ) -> Vec<utoipa::openapi::path::Parameter> {
        use utoipa::openapi::{path::ParameterBuilder, ObjectBuilder};
        let ident = ObjectBuilder::new()
            .schema_type(utoipa::openapi::schema::Type::String)
            .build();
        let limit = ObjectBuilder::new()
            .schema_type(utoipa::openapi::schema::Type::Integer)
            .minimum(Some(0))
            .maximum(Some(1024))
            .default(Some(10.into()))
            .build();
        let dir = ObjectBuilder::new()
            .schema_type(utoipa::openapi::schema::Type::String)
            .enum_values(Some(["b", "f"]))
            .build();
        vec![
            ParameterBuilder::new()
                .name("from")
                .parameter_in(parameter_in_provider().unwrap_or_default())
                .schema(Some(ident.clone()))
                .build(),
            ParameterBuilder::new()
                .name("to")
                .parameter_in(parameter_in_provider().unwrap_or_default())
                .schema(Some(ident))
                .build(),
            ParameterBuilder::new()
                .name("dir")
                .parameter_in(parameter_in_provider().unwrap_or_default())
                .schema(Some(dir))
                .build(),
            ParameterBuilder::new()
                .name("limit")
                .parameter_in(parameter_in_provider().unwrap_or_default())
                .schema(Some(limit))
                .build(),
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum PaginationDirection {
    #[default]
    F,
    B,
}

impl Display for PaginationDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PaginationDirection::F => write!(f, "f"),
            PaginationDirection::B => write!(f, "b"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PaginationResponse<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub has_more: bool,
    pub cursor: Option<String>,
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

/// a set of changes
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
    pub room_member: Vec<RoomMember>,
}