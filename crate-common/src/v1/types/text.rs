#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

/// a language
// TODO: determine which format to use. probably either ietf bcp-47 or a custom enum.
// does this include programming languages?
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Language(pub String);

impl From<String> for Language {
    fn from(value: String) -> Self {
        Self(value)
    }
}
