use std::fmt::Display;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::Identifier;

#[derive(Debug, Deserialize, ToSchema)]
pub struct PaginationQuery<I: Identifier> {
    pub from: Option<I>,
    pub to: Option<I>,
    pub dir: Option<PaginationDirection>,
    pub limit: Option<u16>,
}

#[derive(Debug, Deserialize, Default, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum PaginationDirection {
    #[default]
    F,
    B,
}

impl Display for PaginationDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaginationDirection::F => write!(f, "f"),
            PaginationDirection::B => write!(f, "b"),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginationResponse<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub has_more: bool,
}
