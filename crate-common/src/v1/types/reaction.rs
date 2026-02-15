use std::{fmt::Display, str::FromStr};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{emoji::EmojiCustom, util::Time, EmojiId};

use super::UserId;

/// the total reaction counts for all keys
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ReactionCounts(pub Vec<ReactionCount>);

/// the total reaction counts for a key
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ReactionCount {
    pub key: ReactionKey,
    pub count: u64,

    #[cfg_attr(feature = "serde", serde(default, rename = "self"))]
    pub self_reacted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ReactionListItem {
    pub user_id: UserId,
    pub created_at: Time,
}

/// reaction key returned in reaction counts for messages
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema), serde(tag = "type"))]
pub enum ReactionKey {
    Text { content: String },
    Custom(EmojiCustom),
}

/// reaction key used in reaction route params
///
/// serialized as:
/// - `t:{unicode emoji or text}` for emoji and text
/// - `c:{custom emoji id}` for custom emoji
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ReactionKeyParam {
    Text(String),
    Custom(EmojiId),
}

#[cfg(feature = "utoipa")]
mod u {
    use utoipa::{PartialSchema, ToSchema};

    use super::ReactionKeyParam;

    impl ToSchema for ReactionKeyParam {}

    impl PartialSchema for ReactionKeyParam {
        fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
            utoipa::openapi::ObjectBuilder::new()
                .schema_type(utoipa::openapi::schema::Type::String)
                .description(Some(
                    "Reaction key used in reaction route params.\n\nSerialized as:\n- `t:{unicode emoji or text}` for emoji and text\n- `c:{custom emoji id}` for custom emoji",
                )).examples(vec!["t:🤔".to_owned()])
                .build()
                .into()
        }
    }
}

#[cfg(feature = "serde")]
mod s {
    use std::str::FromStr;

    use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

    use super::ReactionKeyParam;

    impl Serialize for ReactionKeyParam {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.serialize_str(&self.to_string())
        }
    }

    impl<'de> Deserialize<'de> for ReactionKeyParam {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let s = String::deserialize(deserializer)?;
            ReactionKeyParam::from_str(&s)
                .map_err(|_| de::Error::custom("invalid reaction key param format"))
        }
    }
}

impl Display for ReactionKeyParam {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReactionKeyParam::Text(t) => write!(f, "t:{t}"),
            ReactionKeyParam::Custom(id) => write!(f, "c:{id}"),
        }
    }
}

impl FromStr for ReactionKeyParam {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(s) = s.strip_prefix("t:") {
            Ok(ReactionKeyParam::Text(s.to_owned()))
        } else if let Some(s) = s.strip_prefix("c:") {
            Ok(ReactionKeyParam::Custom(s.parse().map_err(|_| ())?))
        } else {
            Err(())
        }
    }
}

impl From<ReactionKey> for ReactionKeyParam {
    fn from(value: ReactionKey) -> Self {
        value.to_param()
    }
}

impl ReactionKey {
    /// get this key as a ReactionKeyParam
    pub fn to_param(&self) -> ReactionKeyParam {
        match self {
            ReactionKey::Text { content: t } => ReactionKeyParam::Text(t.to_owned()),
            ReactionKey::Custom(e) => ReactionKeyParam::Custom(e.id),
        }
    }

    /// get this key as a ReactionKeyParam string
    pub fn to_key_str(&self) -> String {
        self.to_param().to_string()
    }
}
