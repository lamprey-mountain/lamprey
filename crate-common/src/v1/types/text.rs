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

// TODO: use this?
/// a piece of human provided text
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(transparent))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Text(String);

impl Text {
    pub fn new(s: String) -> Self {
        todo!()
    }

    /// get this text's length in practice
    ///
    /// - counts the number of graphemes
    /// - custom emoji counts as a single grapheme
    /// - mentions count as 16 graphemes (arbitrary number, maybe fine tune later?)
    pub fn text_len(&self) -> usize {
        todo!()
    }

    /// get this text's length in bytes
    pub fn byte_len(&self) -> usize {
        todo!()
    }
}

// TODO: impl deref target = str
// TODO: impl from string
