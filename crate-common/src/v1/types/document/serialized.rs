//! json serialized document format

// TODO: actually flesh out the serialized document format

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::components::ComponentCanonical;

/// serialized document
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Serdoc {
    pub components: Vec<ComponentCanonical>,
}
