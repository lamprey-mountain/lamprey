use core::fmt;
use std::{ops::Deref, str::FromStr};
use strum::{Display, EnumString};

#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::v1::types::error::{ApiError, ApiResult, ErrorCode, ErrorField, ErrorFieldType};

/// a color
///
/// ## valid formats
///
/// - hex codes: `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa` (css compatible)
/// - rgb: `rgb(r, g, b)`. with alpha `rgba(r, g, b, a)` (css compatible)
/// - oklch: `oklch(l% c h)`. with alpha `oklch(l% c h / a)` (css compatible)
/// - named: `name`, `name-variant`. with alpha `name:.5`, `name-variant:0.6` (css compatible)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Color {
    /// sRGB (not linear) compatible with css
    ///
    /// may optionally have alpha
    Srgb(ColorSrgb),

    /// oklch color compatible with css
    ///
    /// may optionally have alpha
    Oklch(ColorOklch),

    /// named color with variant
    ///
    /// may optionally have a variant selector and alpha
    Named(ColorNamed),

    /// due to poor validation in the past, there may be invalid data in the database
    ///
    /// rather than return an error, return it as a mystery string. this will be removed later.
    Mystery(String),
}

/// a named (builtin) color
///
/// **COLOR NAMES CURRENTLY UNSTABLE AND MAY CHANGE**
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, Display)]
#[strum(serialize_all = "kebab-case")]
pub enum ColorName {
    /// ui: default text color
    Foreground,

    /// ui: background color
    Background,

    /// themed: default accent color
    Accent,

    /// themed: red
    Red,

    /// themed: green
    Green,

    /// themed: yellow
    Yellow,

    /// themed: blue
    Blue,

    /// themed: magenta
    Magenta,

    /// themed: cyan
    Cyan,

    /// themed: orange
    Orange,

    /// themed: teal
    Teal,

    /// semantic: something worth pointing out
    Note,

    /// semantic: something with useful information
    Info,

    /// semantic: instructions or tips
    Help,

    /// semantic: very important to read, generic
    Important,

    /// semantic: very important to read, bad things may happen if you don't
    Warning,

    /// semantic: very important to read, dangerous things happen if you don't
    Danger,

    /// semantic: something went wrong
    Error,

    /// semantic: something went right
    Success,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorNamed {
    name: ColorName,
    variant: ColorVariant,
    alpha: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorSrgb {
    r: u8,
    g: u8,
    b: u8,
    alpha: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorOklch {
    l: f32,             // 0.0 to 1.0 (or 0-100%)
    c: f32,             // 0.0 to ~0.4
    h: f32,             // 0.0 to 360.0
    alpha: Option<f32>, // 0.0 to 1.0
}

// ColorOklch should never have NaN
impl Eq for ColorOklch {}

/// a color variant
///
/// must be the number `100`, `200`, ..., `900`. defaults to `500`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorVariant(u16);

impl ColorVariant {
    /// create a new validated ColorVariant
    pub fn new(variant: u16) -> ApiResult<Self> {
        if matches!(variant, 100 | 200 | 300 | 400 | 500 | 600 | 700 | 800 | 900) {
            Ok(Self(variant))
        } else {
            Err(ApiError {
                fields: vec![ErrorField {
                    key: vec![],
                    message: format!("Invalid color variant `{variant}`"),
                    ty: ErrorFieldType::Other,
                }],
                ..ApiError::from_code(ErrorCode::InvalidData)
            })
        }
    }

    /// create a new `ColorVariant` without validation
    pub fn new_unchecked(variant: u16) -> Self {
        Self(variant)
    }

    /// get the variant value
    pub fn value(&self) -> u16 {
        self.0
    }
}

impl Default for ColorVariant {
    fn default() -> Self {
        Self(500)
    }
}

impl Deref for ColorVariant {
    type Target = u16;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Color {
    pub fn into_string(self) -> String {
        self.to_string()
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Color::Srgb(c) => write!(f, "{}", c),
            Color::Oklch(c) => write!(f, "{}", c),
            Color::Named(c) => write!(f, "{}", c),
            Color::Mystery(s) => write!(f, "{}", s),
        }
    }
}

impl fmt::Display for ColorSrgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(a) = self.alpha {
            write!(f, "#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, a)
        } else {
            write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        }
    }
}

impl fmt::Display for ColorOklch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(a) = self.alpha {
            write!(
                f,
                "oklch({:.0}% {:.3} {:.2} / {:.2})",
                self.l * 100.0,
                self.c,
                self.h,
                a
            )
        } else {
            write!(
                f,
                "oklch({:.0}% {:.3} {:.2})",
                self.l * 100.0,
                self.c,
                self.h
            )
        }
    }
}

impl fmt::Display for ColorNamed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name.to_string())?;
        if self.variant.value() != 500 {
            write!(f, "-{}", self.variant.value())?;
        }
        if let Some(a) = self.alpha {
            write!(f, ":{:.2}", a as f32 / 255.0)?;
        }
        Ok(())
    }
}

impl Color {
    /// parse a color from a string, disallowing `Mystery` colors
    pub fn from_str_strict(s: &str) -> ApiResult<Self> {
        let s = s.trim();
        if s.is_empty() {
            return Err(ApiError::from_code(ErrorCode::InvalidData));
        }

        if s.starts_with('#') || s.starts_with("rgb") {
            if let Ok(c) = ColorSrgb::from_str(s) {
                return Ok(Color::Srgb(c));
            }
        } else if s.starts_with("oklch") {
            if let Ok(c) = ColorOklch::from_str(s) {
                return Ok(Color::Oklch(c));
            }
        } else {
            if let Ok(c) = ColorNamed::from_str(s) {
                return Ok(Color::Named(c));
            }
        }

        Err(ApiError::from_code(ErrorCode::InvalidData))
    }
}

impl FromStr for Color {
    type Err = ApiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err(ApiError::from_code(ErrorCode::InvalidData));
        }

        if s.starts_with('#') || s.starts_with("rgb") {
            if let Ok(c) = ColorSrgb::from_str(s) {
                return Ok(Color::Srgb(c));
            }
        } else if s.starts_with("oklch") {
            if let Ok(c) = ColorOklch::from_str(s) {
                return Ok(Color::Oklch(c));
            }
        } else {
            if let Ok(c) = ColorNamed::from_str(s) {
                return Ok(Color::Named(c));
            }
        }

        Ok(Color::Mystery(s.to_string()))
    }
}

impl FromStr for ColorSrgb {
    type Err = ApiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim().to_lowercase();

        let parse_err = |msg: String| ApiError {
            fields: vec![ErrorField {
                key: vec![],
                message: msg,
                ty: ErrorFieldType::Other,
            }],
            ..ApiError::from_code(ErrorCode::InvalidData)
        };

        if s.starts_with('#') {
            let hex = &s[1..];
            match hex.len() {
                3 | 4 => {
                    let r = u8::from_str_radix(&hex[0..1], 16)
                        .map_err(|_| parse_err("Invalid hex".into()))?
                        * 17;
                    let g = u8::from_str_radix(&hex[1..2], 16)
                        .map_err(|_| parse_err("Invalid hex".into()))?
                        * 17;
                    let b = u8::from_str_radix(&hex[2..3], 16)
                        .map_err(|_| parse_err("Invalid hex".into()))?
                        * 17;
                    let alpha = if hex.len() == 4 {
                        Some(
                            u8::from_str_radix(&hex[3..4], 16)
                                .map_err(|_| parse_err("Invalid hex".into()))?
                                * 17,
                        )
                    } else {
                        None
                    };
                    Ok(ColorSrgb { r, g, b, alpha })
                }
                6 | 8 => {
                    let r = u8::from_str_radix(&hex[0..2], 16)
                        .map_err(|_| parse_err("Invalid hex".into()))?;
                    let g = u8::from_str_radix(&hex[2..4], 16)
                        .map_err(|_| parse_err("Invalid hex".into()))?;
                    let b = u8::from_str_radix(&hex[4..6], 16)
                        .map_err(|_| parse_err("Invalid hex".into()))?;
                    let alpha = if hex.len() == 8 {
                        Some(
                            u8::from_str_radix(&hex[6..8], 16)
                                .map_err(|_| parse_err("Invalid hex".into()))?,
                        )
                    } else {
                        None
                    };
                    Ok(ColorSrgb { r, g, b, alpha })
                }
                _ => Err(parse_err(
                    "Hex color must be 3, 4, 6, or 8 characters".into(),
                )),
            }
        } else if s.starts_with("rgb") {
            let inner = s
                .split('(')
                .nth(1)
                .and_then(|s| s.split(')').next())
                .ok_or_else(|| parse_err("Malformed rgb()".into()))?;
            let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();

            if parts.len() < 3 {
                return Err(parse_err("rgb() requires at least 3 components".into()));
            }

            let r = parts[0]
                .parse()
                .map_err(|_| parse_err("Invalid R".into()))?;
            let g = parts[1]
                .parse()
                .map_err(|_| parse_err("Invalid G".into()))?;
            let b = parts[2]
                .parse()
                .map_err(|_| parse_err("Invalid B".into()))?;
            let alpha = if parts.len() == 4 {
                if parts[3].contains('.') {
                    let a_f: f32 = parts[3]
                        .parse()
                        .map_err(|_| parse_err("Invalid alpha".into()))?;
                    Some((a_f.clamp(0.0, 1.0) * 255.0) as u8)
                } else {
                    Some(
                        parts[3]
                            .parse()
                            .map_err(|_| parse_err("Invalid alpha".into()))?,
                    )
                }
            } else {
                None
            };
            Ok(ColorSrgb { r, g, b, alpha })
        } else {
            Err(parse_err("Not a valid sRGB format".into()))
        }
    }
}

impl FromStr for ColorOklch {
    type Err = ApiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parse_err = |msg: &str| ApiError {
            fields: vec![ErrorField {
                key: vec![],
                message: msg.to_string(),
                ty: ErrorFieldType::Other,
            }],
            ..ApiError::from_code(ErrorCode::InvalidData)
        };

        let inner = s
            .strip_prefix("oklch(")
            .and_then(|s| s.strip_suffix(')'))
            .ok_or_else(|| parse_err("Malformed oklch()"))?;

        let (main, alpha_part) = inner
            .split_once('/')
            .map(|(m, a)| (m, Some(a.trim())))
            .unwrap_or((inner, None));
        let parts: Vec<&str> = main.split_whitespace().collect();

        if parts.len() != 3 {
            return Err(parse_err("oklch requires L C H"));
        }

        // Parse L (handle both 0.7 and 70%)
        let l_raw = parts[0];
        let l = if l_raw.ends_with('%') {
            l_raw
                .trim_end_matches('%')
                .parse::<f32>()
                .map_err(|_| parse_err("Invalid L"))?
                / 100.0
        } else {
            l_raw.parse::<f32>().map_err(|_| parse_err("Invalid L"))?
        };

        let c = parts[1]
            .parse::<f32>()
            .map_err(|_| parse_err("Invalid C"))?;
        let h = parts[2]
            .parse::<f32>()
            .map_err(|_| parse_err("Invalid H"))?;

        let alpha = if let Some(a_str) = alpha_part {
            Some(
                a_str
                    .parse::<f32>()
                    .map_err(|_| parse_err("Invalid alpha"))?,
            )
        } else {
            None
        };

        Ok(ColorOklch { l, c, h, alpha })
    }
}

impl FromStr for ColorNamed {
    type Err = ApiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (base, alpha_part) = s
            .split_once(':')
            .map(|(b, a)| (b, Some(a)))
            .unwrap_or((s, None));
        let (name_part, variant_part) = base
            .split_once('-')
            .map(|(n, v)| (n, Some(v)))
            .unwrap_or((base, None));

        let name = ColorName::from_str(name_part).map_err(|_| ApiError {
            fields: vec![ErrorField {
                key: vec![],
                message: format!("Unknown color name '{name_part}'"),
                ty: ErrorFieldType::Other,
            }],
            ..ApiError::from_code(ErrorCode::InvalidData)
        })?;

        let variant = if let Some(v_str) = variant_part {
            let v_u16 = v_str
                .parse()
                .map_err(|_| ApiError::from_code(ErrorCode::InvalidData))?;
            ColorVariant::new(v_u16)?
        } else {
            ColorVariant::default()
        };

        let alpha = if let Some(a_str) = alpha_part {
            let a_f: f32 = a_str
                .parse()
                .map_err(|_| ApiError::from_code(ErrorCode::InvalidData))?;
            Some((a_f.clamp(0.0, 1.0) * 255.0) as u8)
        } else {
            None
        };

        Ok(ColorNamed {
            name,
            variant,
            alpha,
        })
    }
}

macro_rules! impl_serde_via_string {
    ($t:ty) => {
        impl Serialize for $t {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(&self.to_string())
            }
        }

        impl<'de> Deserialize<'de> for $t {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let s = String::deserialize(deserializer)?;
                Self::from_str(&s).map_err(serde::de::Error::custom)
            }
        }
    };
}

impl_serde_via_string!(Color);
impl_serde_via_string!(ColorSrgb);
impl_serde_via_string!(ColorOklch);
impl_serde_via_string!(ColorNamed);

impl Serialize for ColorVariant {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u16(self.0)
    }
}

impl<'de> Deserialize<'de> for ColorVariant {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let val = u16::deserialize(deserializer)?;
        Self::new(val).map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "utoipa")]
mod _u {
    use utoipa::{
        PartialSchema, ToSchema,
        openapi::{ObjectBuilder, schema::Schema},
    };

    use crate::v1::types::misc::Color;

    impl PartialSchema for Color {
        fn schema() -> utoipa::openapi::RefOr<Schema> {
            ObjectBuilder::new()
                .schema_type(utoipa::openapi::schema::Type::String)
                .description(Some("A color string. Supports hex (#rgb, #rrggbb), rgb(), oklch(), or named colors (name-variant:alpha)"))
                .examples([
                    serde_json::json!("red-500"),
                    serde_json::json!("#ff0000"),
                    serde_json::json!("oklch(70% 0.1 120)"),
                ])
                .build()
                .into()
        }
    }

    impl ToSchema for Color {}
}

#[cfg(test)]
mod test;
