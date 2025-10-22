use core::ops::Deref;
use std::{fmt::Display, str::FromStr};

use mediatype::{MediaTypeBuf, MediaTypeError};
use serde::{Deserialize, Serialize};

/// A mime/media type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mime(String);

impl Mime {
    pub fn parse(&self) -> Result<MediaTypeBuf, MediaTypeError> {
        self.0.parse()
    }
}

impl FromStr for Mime {
    type Err = MediaTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Validate the string can be parsed into a MediaTypeBuf
        s.parse::<MediaTypeBuf>()?;
        Ok(Self(s.to_string()))
    }
}

impl Deref for Mime {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for Mime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(feature = "utoipa")]
mod schema {
    use utoipa::{
        openapi::{RefOr, Schema},
        schema, PartialSchema, ToSchema,
    };

    use super::Mime;

    impl PartialSchema for Mime {
        fn schema() -> RefOr<Schema> {
            let schema = schema!(String)
                .title(Some("Mime"))
                .description(Some("a mime/media type"))
                .examples([serde_json::json!("application/json")])
                .build();
            RefOr::T(Schema::Object(schema))
        }
    }

    impl ToSchema for Mime {}
}
