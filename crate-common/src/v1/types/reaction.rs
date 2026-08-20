use std::{fmt::Display, str::FromStr};

use lamprey_macros::record;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "feat_tag_reactions")]
use crate::v1::types::tag::Tag;
use crate::v1::types::{EmojiId, emoji::EmojiCustom, util::Time};

use super::{MessageId, UserId};

/// A reaction with its creation timestamp, returned in sync events.
#[record]
pub struct Reaction {
    #[serde(flatten)]
    pub info: ReactionInfo,
    pub created_at: Time,
}

/// Information identifying a reaction without its timestamp.
#[record]
pub struct ReactionInfo {
    pub user_id: UserId,
    pub message_id: MessageId,
    pub key: ReactionKey,
}

/// the total reaction counts for all keys
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ReactionCounts(pub Vec<ReactionCount>);

/// the total reaction counts for a key
#[record]
#[derive(PartialEq, Eq)]
pub struct ReactionCount {
    pub key: ReactionKey,
    pub count: u64,

    #[serde(default, rename = "self")]
    pub self_reacted: bool,
}

#[record]
#[derive(PartialEq, Eq)]
pub struct ReactionListItem {
    pub user_id: UserId,
    pub created_at: Time,
}

/// reaction key returned in reaction counts for messages
#[record]
#[derive(PartialEq, Eq)]
#[serde(tag = "type")]
pub enum ReactionKey {
    Text {
        content: String,
    },
    Custom(EmojiCustom),
    #[cfg(feature = "feat_tag_reactions")]
    Tag(Tag),
}

/// reaction key used in reaction route params
///
/// serialized as:
/// - `t:{unicode emoji or text}` for emoji and text
/// - `c:{custom emoji id}` for custom emoji
/// - `a:{tag id}` for thread tags
// TODO: redo string serialization format?
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ReactionKeyParam {
    Text(String),
    Custom(EmojiId),
    #[cfg(feature = "feat_tag_reactions")]
    Tag(TagId),
}

/// deserializes as a `ReactionKeyParam`, serializes as a `ReactionKey`
// TODO: use this?
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ReactionKeyField {
    Param(ReactionKeyParam),
    Key(ReactionKey),
}

impl ReactionKeyField {
    // pub fn a(&self) {}
}

#[cfg(feature = "utoipa")]
mod u {
    use utoipa::{PartialSchema, ToSchema};

    use super::{ReactionKeyField, ReactionKeyParam};

    impl ToSchema for ReactionKeyParam {}

    impl ToSchema for ReactionKeyField {}

    impl PartialSchema for ReactionKeyParam {
        fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
            utoipa::openapi::ObjectBuilder::new()
                .schema_type(utoipa::openapi::schema::Type::String)
                .description(Some(
                    "Reaction key used in reaction route params.\n\nSerialized as:\n- `t:{unicode emoji or text}` for emoji and text\n- `c:{custom emoji id}` for custom emoji\n- `a:{tag id}` for thread tags",
                )).examples(vec!["t:🤔".to_owned()])
                .build()
                .into()
        }
    }

    impl PartialSchema for ReactionKeyField {
        fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
            ReactionKeyParam::schema()
        }
    }
}

#[cfg(feature = "serde")]
mod s {
    use std::str::FromStr;

    use serde::{Deserialize, Deserializer, Serialize, Serializer, de, ser};

    use super::{ReactionKeyField, ReactionKeyParam};

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

    impl Serialize for ReactionKeyField {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            match self {
                ReactionKeyField::Key(key) => key.serialize(serializer),
                ReactionKeyField::Param(_) => Err(ser::Error::custom(
                    "ReactionKeyField::Param cannot be serialized",
                )),
            }
        }
    }

    impl<'de> Deserialize<'de> for ReactionKeyField {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            ReactionKeyParam::deserialize(deserializer).map(ReactionKeyField::Param)
        }
    }
}

impl Display for ReactionKeyParam {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReactionKeyParam::Text(t) => write!(f, "t:{t}"),
            ReactionKeyParam::Custom(id) => write!(f, "c:{id}"),
            #[cfg(feature = "feat_tag_reactions")]
            ReactionKeyParam::Tag(id) => write!(f, "a:{id}"),
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
            #[cfg(feature = "feat_tag_reactions")]
            if let Some(s) = s.strip_prefix("a:") {
                return Ok(ReactionKeyParam::Tag(s.parse().map_err(|_| ())?));
            }

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
    /// check if this key is a custom emoji
    pub fn is_custom_emoji(&self) -> bool {
        matches!(self, ReactionKey::Custom(_))
    }

    #[cfg(feature = "feat_tag_reactions")]
    /// check if this key is a thread tag
    pub fn is_thread_tag(&self) -> bool {
        matches!(self, ReactionKey::Tag(_))
    }

    /// check if this key is a single unicode emoji
    ///
    /// this only returns true if the content is a single visual emoji,
    /// including modifiers like skin tones or hair colors.
    pub fn is_unicode_emoji(&self) -> bool {
        match self {
            ReactionKey::Text { content } => {
                use unicode_properties::UnicodeEmoji;
                use unicode_segmentation::UnicodeSegmentation;

                let mut graphemes = content.graphemes(true);
                let Some(g) = graphemes.next() else {
                    return false;
                };

                // must be exactly one grapheme cluster
                if graphemes.next().is_some() {
                    return false;
                }

                // an emoji grapheme cluster should contain at least one emoji character.
                // we use is_emoji_char_or_emoji_component() to catch base emojis and their modifiers.
                // we also ensure it's not just a plain alphanumeric string (like "1" or "A")
                // that happens to be a single grapheme.
                g.chars().any(|c| c.is_emoji_char_or_emoji_component())
                    && !g.chars().all(|c| c.is_ascii_alphanumeric())
            }
            ReactionKey::Custom(_) => false,
            #[cfg(feature = "feat_tag_reactions")]
            ReactionKey::Tag(_) => false,
        }
    }

    /// check if this key is an emoji (custom or unicode)
    pub fn is_emoji(&self) -> bool {
        self.is_custom_emoji() || self.is_unicode_emoji()
    }

    /// get this key as a ReactionKeyParam
    pub fn to_param(&self) -> ReactionKeyParam {
        match self {
            ReactionKey::Text { content: t } => ReactionKeyParam::Text(t.to_owned()),
            ReactionKey::Custom(e) => ReactionKeyParam::Custom(e.id),
            #[cfg(feature = "feat_tag_reactions")]
            ReactionKey::Tag(t) => ReactionKeyParam::Tag(t.id),
        }
    }

    /// get this key as a ReactionKeyParam string
    pub fn to_key_str(&self) -> String {
        self.to_param().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_unicode_emoji() {
        let cases = [
            ("👍", true),
            ("👍🏽", true), // skin tone
            ("👨‍👩‍👧‍👦", true), // ZWJ sequence
            ("A", false),
            ("1", false),
            ("1️⃣", true), // keycap
            ("🤔", true),
            ("hello", false),
            ("!!", false),
            ("❤️", true), // variant selector
            ("♥️", true),
        ];

        for (content, expected) in cases {
            let key = ReactionKey::Text {
                content: content.to_string(),
            };
            assert_eq!(
                key.is_unicode_emoji(),
                expected,
                "failed for: {content} (expected: {expected})"
            );
        }
    }
}
