use std::{fmt::Display, str::FromStr};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// a set of ratings for a piece of content
///
/// `disposition(type type) disposition(type) type` etc...
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentRatings {
    ratings: Vec<(Option<ContentRatingDisposition>, Vec<ContentRatingType>)>,
}

#[derive(Debug, Clone, PartialEq, Eq, strum::Display, strum::EnumString)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ContentRatingDisposition {
    #[strum(serialize = "irl.mild")]
    #[cfg_attr(feature = "serde", serde(rename = "irl.mild"))]
    IrlMild,

    #[strum(serialize = "irl.severe")]
    #[cfg_attr(feature = "serde", serde(rename = "irl.severe"))]
    IrlSevere,

    #[strum(serialize = "fiction.mild")]
    #[cfg_attr(feature = "serde", serde(rename = "fiction.mild"))]
    FictionMild,

    #[strum(serialize = "fiction.severe")]
    #[cfg_attr(feature = "serde", serde(rename = "fiction.severe"))]
    FictionSevere,

    #[strum(serialize = "suggestive")]
    #[cfg_attr(feature = "serde", serde(rename = "suggestive"))]
    Suggestive,

    /// other unknown disposition
    #[cfg_attr(feature = "serde", serde(untagged))]
    #[strum(default)]
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, strum::Display, strum::EnumString)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(rename_all = "lowercase")
)]
#[strum(serialize_all = "lowercase")]
pub enum ContentRatingType {
    /// contains nudity
    Nudity,

    /// contains sexual content. usually but not always paired with `Nudity`
    Sexual,

    /// contains violence
    Violence,

    /// contains gore
    Gore,

    /// contains strong language (profanity)
    Language,

    /// contains spoilers
    Spoiler,

    /// advisory for users with photosensitivity (flashing lights, rapid movement, etc)
    Photosensitivity,

    /// is very loud (headphone users warning)
    Loud,

    /// other unknown content rating
    #[cfg_attr(feature = "serde", serde(untagged))]
    #[strum(default)]
    Other(String),
}

// TODO: derive error, etc
// TODO: use this
pub enum ContentRatingsParseError {
    /// got an unexpected character
    UnexpectedCharacter { got: char, message: String },
}

// // TODO: use logos?
// enum ContentRatingsToken {
//     OpenParan,
//     CloseParan,
//     Token,
// }

impl Display for ContentRatings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for (disp, tys) in &self.ratings {
            if !first {
                write!(f, " ")?;
            }
            first = false;

            if let Some(d) = disp {
                write!(f, "{}(", d)?;
                for (i, t) in tys.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", t)?;
                }
                write!(f, ")")?;
            } else {
                for (i, t) in tys.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", t)?;
                }
            }
        }
        Ok(())
    }
}

impl FromStr for ContentRatings {
    // TODO: better error type
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut ratings = Vec::new();
        let tokens: Vec<&str> = s.split_whitespace().collect();
        let mut i = 0;

        while i < tokens.len() {
            let token = tokens[i];
            if let Some(paren_idx) = token.find('(') {
                let disp_str = &token[..paren_idx];
                let disp = ContentRatingDisposition::from_str(disp_str)
                    .map_err(|_| format!("Invalid disposition: {}", disp_str))?;

                let mut group_types = Vec::new();
                let first_type_part = &token[paren_idx + 1..];

                if let Some(end_idx) = first_type_part.find(')') {
                    group_types
                        .push(ContentRatingType::from_str(&first_type_part[..end_idx]).unwrap());
                } else {
                    group_types.push(ContentRatingType::from_str(first_type_part).unwrap());
                    i += 1;
                    while i < tokens.len() {
                        if let Some(end_idx) = tokens[i].find(')') {
                            group_types
                                .push(ContentRatingType::from_str(&tokens[i][..end_idx]).unwrap());
                            break;
                        } else {
                            group_types.push(ContentRatingType::from_str(tokens[i]).unwrap());
                        }
                        i += 1;
                    }
                }
                ratings.push((Some(disp), group_types));
            } else {
                ratings.push((None, vec![ContentRatingType::from_str(token).unwrap()]));
            }
            i += 1;
        }
        Ok(ContentRatings { ratings })
    }
}

#[cfg(feature = "serde")]
mod _s {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use crate::v1::types::headers::ContentRatings;

    impl Serialize for ContentRatings {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.serialize_str(&self.to_string())
        }
    }

    impl<'de> Deserialize<'de> for ContentRatings {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let s = String::deserialize(deserializer)?;
            s.parse().map_err(serde::de::Error::custom)
        }
    }
}

#[cfg(feature = "utoipa")]
mod _u {
    use utoipa::{
        openapi::{schema::Schema, ObjectBuilder},
        PartialSchema, ToSchema,
    };

    use crate::v1::types::headers::ContentRatings;

    impl PartialSchema for ContentRatings {
        fn schema() -> utoipa::openapi::RefOr<Schema> {
            ObjectBuilder::new()
                .schema_type(utoipa::openapi::schema::Type::String)
                .description(Some(
                    "A content rating string. Format: `disposition(type) type` etc...",
                ))
                .examples([
                    serde_json::json!("irl.mild(nudity sexual)"),
                    serde_json::json!("fiction.mild(violence)"),
                    serde_json::json!("suggestive"),
                ])
                .build()
                .into()
        }
    }

    impl ToSchema for ContentRatings {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn rating_type_display() {
        assert_eq!(ContentRatingType::Nudity.to_string(), "nudity");
        assert_eq!(ContentRatingType::Sexual.to_string(), "sexual");
        assert_eq!(ContentRatingType::Violence.to_string(), "violence");
        assert_eq!(ContentRatingType::Gore.to_string(), "gore");
        assert_eq!(ContentRatingType::Language.to_string(), "language");
        assert_eq!(ContentRatingType::Spoiler.to_string(), "spoiler");
        assert_eq!(
            ContentRatingType::Photosensitivity.to_string(),
            "photosensitivity"
        );
        assert_eq!(ContentRatingType::Loud.to_string(), "loud");
        assert_eq!(
            ContentRatingType::Other("custom".into()).to_string(),
            "custom"
        );
    }

    #[test]
    fn rating_type_from_str() {
        assert_eq!(
            ContentRatingType::from_str("nudity").unwrap(),
            ContentRatingType::Nudity
        );
        ContentRatingType::from_str("NUDITY").unwrap_err();
        ContentRatingType::from_str("Nudity").unwrap_err();
        assert_eq!(
            ContentRatingType::from_str("photosensitivity").unwrap(),
            ContentRatingType::Photosensitivity
        );
        assert_eq!(
            ContentRatingType::from_str("unknown-thing").unwrap(),
            ContentRatingType::Other("unknown-thing".into())
        );
    }

    #[test]
    fn rating_type_roundtrip() {
        let variants = [
            ContentRatingType::Nudity,
            ContentRatingType::Sexual,
            ContentRatingType::Violence,
            ContentRatingType::Gore,
            ContentRatingType::Language,
            ContentRatingType::Spoiler,
            ContentRatingType::Photosensitivity,
            ContentRatingType::Loud,
            ContentRatingType::Other("custom".into()),
        ];
        for v in &variants {
            assert_eq!(ContentRatingType::from_str(&v.to_string()).unwrap(), *v);
        }
    }

    #[test]
    fn disposition_display() {
        assert_eq!(ContentRatingDisposition::IrlMild.to_string(), "irl.mild");
        assert_eq!(
            ContentRatingDisposition::IrlSevere.to_string(),
            "irl.severe"
        );
        assert_eq!(
            ContentRatingDisposition::FictionMild.to_string(),
            "fiction.mild"
        );
        assert_eq!(
            ContentRatingDisposition::FictionSevere.to_string(),
            "fiction.severe"
        );
        assert_eq!(
            ContentRatingDisposition::Suggestive.to_string(),
            "suggestive"
        );
    }

    #[test]
    fn disposition_from_str() {
        assert_eq!(
            ContentRatingDisposition::from_str("irl.mild").unwrap(),
            ContentRatingDisposition::IrlMild
        );
        assert_eq!(
            ContentRatingDisposition::from_str("fiction.severe").unwrap(),
            ContentRatingDisposition::FictionSevere
        );
        assert!(ContentRatingDisposition::from_str("unknown").is_err());
    }

    #[test]
    fn disposition_roundtrip() {
        let variants = [
            ContentRatingDisposition::IrlMild,
            ContentRatingDisposition::IrlSevere,
            ContentRatingDisposition::FictionMild,
            ContentRatingDisposition::FictionSevere,
            ContentRatingDisposition::Suggestive,
        ];
        for v in &variants {
            assert_eq!(
                ContentRatingDisposition::from_str(&v.to_string()).unwrap(),
                *v
            );
        }
    }

    fn cr(
        ratings: Vec<(Option<ContentRatingDisposition>, Vec<ContentRatingType>)>,
    ) -> ContentRatings {
        ContentRatings { ratings }
    }

    #[test]
    fn content_ratings_empty() {
        let parsed = ContentRatings::from_str("").unwrap();
        assert_eq!(parsed, cr(vec![]));
        assert_eq!(parsed.to_string(), "");
    }

    #[test]
    fn content_ratings_bare_single() {
        let parsed = ContentRatings::from_str("violence").unwrap();
        assert_eq!(parsed, cr(vec![(None, vec![ContentRatingType::Violence])]));
        assert_eq!(parsed.to_string(), "violence");
    }

    #[test]
    fn content_ratings_bare_multiple() {
        let parsed = ContentRatings::from_str("violence language").unwrap();
        assert_eq!(
            parsed,
            cr(vec![(
                None,
                vec![ContentRatingType::Violence, ContentRatingType::Language]
            )])
        );
        assert_eq!(parsed.to_string(), "violence language");
    }

    #[test]
    fn content_ratings_with_disposition() {
        let parsed = ContentRatings::from_str("irl.mild(nudity sexual)").unwrap();
        assert_eq!(
            parsed,
            cr(vec![(
                Some(ContentRatingDisposition::IrlMild),
                vec![ContentRatingType::Nudity, ContentRatingType::Sexual]
            ),])
        );
        assert_eq!(parsed.to_string(), "irl.mild(nudity sexual)");
    }

    #[test]
    fn content_ratings_mixed() {
        let parsed =
            ContentRatings::from_str("irl.mild(nudity sexual) violence suggestive(sexual)")
                .unwrap();
        assert_eq!(
            parsed,
            cr(vec![
                (
                    Some(ContentRatingDisposition::IrlMild),
                    vec![ContentRatingType::Nudity, ContentRatingType::Sexual]
                ),
                (None, vec![ContentRatingType::Violence]),
                (
                    Some(ContentRatingDisposition::Suggestive),
                    vec![ContentRatingType::Sexual]
                ),
            ])
        );
        assert_eq!(
            parsed.to_string(),
            "irl.mild(nudity sexual) violence suggestive(sexual)"
        );
    }

    #[test]
    fn content_ratings_unknown_type() {
        let parsed = ContentRatings::from_str("irl.severe(custom-thing)").unwrap();
        assert_eq!(
            parsed,
            cr(vec![(
                Some(ContentRatingDisposition::IrlSevere),
                vec![ContentRatingType::Other("custom-thing".into())]
            ),])
        );
    }

    #[test]
    fn content_ratings_roundtrip() {
        let cases = [
            "violence",
            "violence language",
            "irl.mild(nudity sexual)",
            "irl.mild(nudity sexual) violence suggestive(sexual)",
            "fiction.severe(gore violence) language",
        ];
        for case in &cases {
            let parsed = ContentRatings::from_str(case).unwrap();
            assert_eq!(parsed.to_string(), *case, "roundtrip failed for: {case}");
        }
    }

    #[test]
    fn content_ratings_invalid_disposition() {
        assert!(ContentRatings::from_str("bad.disp(nudity)").is_err());
    }

    #[test]
    fn content_ratings_unclosed_paren() {
        assert!(ContentRatings::from_str("irl.mild(nudity").is_err());
    }
}
