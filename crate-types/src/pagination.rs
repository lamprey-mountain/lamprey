use std::fmt::{self, Display};

use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use super::Identifier;

#[derive(Debug, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PaginationQuery<I: Identifier> {
    pub from: Option<I>,
    pub to: Option<I>,
    pub dir: Option<PaginationDirection>,
    pub limit: Option<u16>,
}

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
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
}
