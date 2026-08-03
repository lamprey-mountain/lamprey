use std::ops::Deref;
use strum::{Display, EnumString};

use crate::v1::types::error::{ApiError, ApiResult, ErrorCode, ErrorField, ErrorFieldType};

mod parse;

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

/// a named builtin color to use from the theme
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

// TODO: hoist alpha to Color? ie:
// pub struct Color {
//     pub kind: ColorKind,
//     pub alpha: Option<f32>,
// }
//
// pub enum ColorKind {
//     Srgb(ColorSrgb),
//     Oklch(ColorOklch),
//     Named(ColorNamed),
//     Mystery(String),
// }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorNamed {
    pub name: ColorName,
    pub variant: ColorVariant,

    // 0.0 to 1.0
    pub alpha: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorSrgb {
    pub r: f32,
    pub g: f32,
    pub b: f32,

    // 0.0 to 1.0
    pub alpha: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorOklch {
    // 0.0 to 1.0 (or 0-100%)
    pub l: f32,

    // 0.0 to ~0.4
    pub c: f32,

    // 0.0 to 360.0
    pub h: f32,

    // 0.0 to 1.0
    pub alpha: Option<f32>,
}

// Colors should never have NaN
impl Eq for ColorNamed {}
impl Eq for ColorSrgb {}
impl Eq for ColorOklch {}

/// a color variant
///
/// must be the number `100`, `200`, ..., `900`. defaults to `500`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorVariant(u16);

impl ColorNamed {
    pub fn alpha(&self) -> Option<f32> {
        match self.alpha {
            Some(1.0) | None => None,
            Some(a) => Some(a),
        }
    }
}

impl ColorSrgb {
    pub fn alpha(&self) -> Option<f32> {
        match self.alpha {
            Some(1.0) | None => None,
            Some(a) => Some(a),
        }
    }
}

impl ColorOklch {
    pub fn alpha(&self) -> Option<f32> {
        match self.alpha {
            Some(1.0) | None => None,
            Some(a) => Some(a),
        }
    }
}

impl ColorVariant {
    /// create a new validated ColorVariant
    pub fn new(variant: u16) -> ApiResult<Self> {
        // NOTE: do i want to restrict to these specific variants?
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
    /// parse a color from a string, disallowing `Mystery` colors
    pub fn from_str_strict(s: &str) -> ApiResult<Self> {
        match s.parse()? {
            Color::Mystery(_) => Err(ApiError::from_code(ErrorCode::InvalidData)),
            c => Ok(c),
        }
    }

    /// returns this color's alpha component
    pub fn alpha(&self) -> Option<f32> {
        match self {
            Color::Srgb(c) => c.alpha(),
            Color::Oklch(c) => c.alpha(),
            Color::Named(c) => c.alpha(),

            // colors in the older api never had alpha
            Color::Mystery(_) => None,
        }
    }

    /// returns whether this color contains an alpha component
    pub fn has_alpha(&self) -> bool {
        self.alpha().is_some()
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
