use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

/// a language
// TODO: determine which format to use. probably either ietf bcp-47 or a custom enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Language(pub String);

/// any piece of text intended for humans to read; only has light formatting
pub struct Text(String);

/// any piece of text intended for humans to read; may be formatted (eg. messages)
/// uses my as of yet unspecced tagged text format
pub struct FormattedText<'a> {
    s: &'a str,
}

impl<'a> FormattedText<'a> {
    pub fn parse(s: &'a str) -> Result<FormattedText<'a>, ()> {
        Ok(Self { s })
    }
}

impl From<String> for Language {
    fn from(value: String) -> Self {
        Self(value)
    }
}
