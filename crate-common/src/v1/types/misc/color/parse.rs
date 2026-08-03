use core::fmt;
use std::str::FromStr;

use thiserror::Error;

use crate::v1::types::{
    error::{ApiError, ErrorCode, ErrorField, ErrorFieldType},
    misc::{
        Color,
        color::{ColorName, ColorNamed, ColorOklch, ColorSrgb, ColorVariant},
    },
};

// TODO: rgb(): support percentages
// TODO: rgb(): support new commaless syntax (ie. rgb(r g b))
// TODO: rgb(): support slash for alpha (ie. rgb(r g b / a))
// TODO: rgba(): add alias for css compat
// TODO: add oklab() function?

/// an error that occured while parsing a color
#[derive(Debug, Clone, Error)]
pub enum ColorParseError {
    /// invalid hex (base 16) value
    #[error("invalid hex at `{at}`")]
    InvalidHex { at: &'static str },

    /// invalid hex format (number of chars)
    #[error("Hex color must be 3, 4, 6, or 8 characters")]
    InvalidHexFormat,

    /// invalid format
    #[error("{message}")]
    // TODO: remove this?
    InvalidFormat { message: String },

    /// invalid float
    #[error("invalid float at `{at}`")]
    InvalidFloat { at: &'static str },

    /// invalid variant
    #[error("invalid variant {s}")]
    InvalidVariant { s: String },

    /// malformed function-style string
    #[error("malformed fn-style string {fn_name}()")]
    MalformedFunction { fn_name: &'static str },

    /// function-style string has invalid number of parameters
    // TODO: show number of params in error display
    #[error("{fn_name}() expects valid number of parameters")]
    FunctionParams {
        fn_name: &'static str,
        param_count: &'static [u8],
    },

    /// unknown color name
    #[error("unknown color name \"{name}\"")]
    UnknownColorName { name: String },

    /// string is empty
    #[error("string is empty")]
    Empty,
}

impl From<ColorParseError> for ApiError {
    fn from(value: ColorParseError) -> Self {
        let message = match value {
            ColorParseError::InvalidHex { at } => format!("Invalid hex component `{at}`"),
            ColorParseError::InvalidHexFormat => {
                "Hex color must be 3, 4, 6, or 8 characters".to_string()
            }
            ColorParseError::InvalidFloat { at } => format!("Invalid float component `{at}`"),
            ColorParseError::MalformedFunction { fn_name } => format!("Malformed {fn_name}()"),
            ColorParseError::FunctionParams {
                fn_name,
                param_count,
            } => {
                // PERF: theres probably some way to optimize this
                let counts: Vec<_> = param_count.iter().map(|c| c.to_string()).collect();
                format!("{fn_name}() expects {} parameters", counts.join(", "))
            }
            ColorParseError::UnknownColorName { name } => format!("Unknown color name '{name}'"),
            ColorParseError::Empty => "Color string is empty".to_string(),
            ColorParseError::InvalidFormat { message } => message,
            ColorParseError::InvalidVariant { s } => {
                format!("invalid variant {s}")
            }
        };
        ApiError {
            fields: vec![ErrorField {
                key: vec![],
                message,
                ty: ErrorFieldType::Other,
            }],
            ..ApiError::from_code(ErrorCode::InvalidData)
        }
    }
}

impl FromStr for Color {
    type Err = ColorParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err(ColorParseError::Empty);
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
    type Err = ColorParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim().to_lowercase();

        if s.starts_with('#') {
            let hex = &s[1..];
            match hex.len() {
                3 | 4 => {
                    let r = u8::from_str_radix(&hex[0..1], 16)
                        .map_err(|_| ColorParseError::InvalidHex { at: "r" })?
                        * 17;
                    let g = u8::from_str_radix(&hex[1..2], 16)
                        .map_err(|_| ColorParseError::InvalidHex { at: "g" })?
                        * 17;
                    let b = u8::from_str_radix(&hex[2..3], 16)
                        .map_err(|_| ColorParseError::InvalidHex { at: "b" })?
                        * 17;
                    let alpha = if hex.len() == 4 {
                        let a = u8::from_str_radix(&hex[3..4], 16)
                            .map_err(|_| ColorParseError::InvalidHex { at: "a" })?
                            * 17;
                        Some(a)
                    } else {
                        None
                    };
                    Ok(ColorSrgb {
                        r: r as f32 / 255.0,
                        g: g as f32 / 255.0,
                        b: b as f32 / 255.0,
                        alpha: alpha.map(|a| a as f32 / 255.0),
                    })
                }
                6 | 8 => {
                    let r = u8::from_str_radix(&hex[0..2], 16)
                        .map_err(|_| ColorParseError::InvalidHex { at: "r" })?;
                    let g = u8::from_str_radix(&hex[2..4], 16)
                        .map_err(|_| ColorParseError::InvalidHex { at: "g" })?;
                    let b = u8::from_str_radix(&hex[4..6], 16)
                        .map_err(|_| ColorParseError::InvalidHex { at: "b" })?;
                    let alpha = if hex.len() == 8 {
                        let a = u8::from_str_radix(&hex[6..8], 16)
                            .map_err(|_| ColorParseError::InvalidHex { at: "a" })?;
                        Some(a)
                    } else {
                        None
                    };
                    Ok(ColorSrgb {
                        r: r as f32 / 255.0,
                        g: g as f32 / 255.0,
                        b: b as f32 / 255.0,
                        alpha: alpha.map(|a| a as f32 / 255.0),
                    })
                }
                _ => Err(ColorParseError::InvalidHexFormat),
            }
        } else if s.starts_with("rgb") {
            let inner = s
                .split('(')
                .nth(1)
                .and_then(|s| s.split(')').next())
                .ok_or_else(|| ColorParseError::MalformedFunction { fn_name: "rgb" })?;

            let (components_str, alpha_str) = if let Some(slash_idx) = inner.find('/') {
                (&inner[..slash_idx], Some(&inner[slash_idx + 1..]))
            } else {
                (inner, None)
            };

            let mut parts: Vec<&str> = if components_str.contains(',') {
                components_str
                    .split(',')
                    .map(|p| p.trim())
                    .filter(|p| !p.is_empty())
                    .collect()
            } else {
                components_str
                    .split_whitespace()
                    .filter(|p| !p.is_empty())
                    .collect()
            };

            // If no slash, check if alpha is in parts
            let alpha_str = if alpha_str.is_none() && parts.len() == 4 {
                Some(parts.remove(3))
            } else {
                alpha_str
            };

            if parts.len() != 3 {
                return Err(ColorParseError::FunctionParams {
                    fn_name: "rgb",
                    param_count: &[3, 4],
                });
            }

            fn parse_component(s: &str, at: &'static str) -> Result<f32, ColorParseError> {
                if s.ends_with('%') {
                    let f: f32 = s
                        .trim_end_matches('%')
                        .parse()
                        .map_err(|_| ColorParseError::InvalidFloat { at })?;
                    if f.is_nan() {
                        return Err(ColorParseError::InvalidFloat { at });
                    }
                    Ok(f / 100.0)
                } else {
                    let f: f32 = s
                        .parse::<f32>()
                        .map_err(|_| ColorParseError::InvalidFloat { at })?;
                    if f.is_nan() {
                        return Err(ColorParseError::InvalidFloat { at });
                    }
                    Ok(f / 255.0)
                }
            }

            fn parse_alpha(s: &str) -> Result<f32, ColorParseError> {
                if s.ends_with('%') {
                    let f: f32 = s
                        .trim_end_matches('%')
                        .parse()
                        .map_err(|_| ColorParseError::InvalidFloat { at: "a" })?;
                    if f.is_nan() {
                        return Err(ColorParseError::InvalidFloat { at: "a" });
                    }
                    Ok(f / 100.0)
                } else {
                    let f: f32 = s
                        .parse::<f32>()
                        .map_err(|_| ColorParseError::InvalidFloat { at: "a" })?;
                    if f.is_nan() {
                        return Err(ColorParseError::InvalidFloat { at: "a" });
                    }
                    Ok(f / 1.0)
                }
            }

            let r = parse_component(parts[0], "r")?;
            let g = parse_component(parts[1], "g")?;
            let b = parse_component(parts[2], "b")?;
            let alpha = alpha_str.map(parse_alpha).transpose()?;

            Ok(ColorSrgb { r, g, b, alpha })
        } else {
            Err(ColorParseError::InvalidFormat {
                message: "Not a valid sRGB format".into(),
            })
        }
    }
}

impl FromStr for ColorOklch {
    type Err = ColorParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let inner = s
            .strip_prefix("oklch(")
            .and_then(|s| s.strip_suffix(')'))
            .ok_or_else(|| ColorParseError::MalformedFunction { fn_name: "oklch" })?;

        let (main, alpha_part) = inner
            .split_once('/')
            .map(|(m, a)| (m, Some(a.trim())))
            .unwrap_or((inner, None));
        let parts: Vec<&str> = main.split_whitespace().collect();

        if parts.len() < 3 || parts.len() > 3 {
            return Err(ColorParseError::FunctionParams {
                fn_name: "oklch",
                param_count: &[3, 4],
            });
        }

        // Parse L (handle both 0.7 and 70%)
        let l_raw = parts[0];
        let l = if l_raw.ends_with('%') {
            l_raw
                .trim_end_matches('%')
                .parse::<f32>()
                .map_err(|_| ColorParseError::InvalidFloat { at: "l" })?
                / 100.0
        } else {
            l_raw
                .parse::<f32>()
                .map_err(|_| ColorParseError::InvalidFloat { at: "l" })?
        };
        if l.is_nan() {
            return Err(ColorParseError::InvalidFloat { at: "l" });
        }

        let c = parts[1]
            .parse::<f32>()
            .map_err(|_| ColorParseError::InvalidFloat { at: "c" })?;
        let h = parts[2]
            .parse::<f32>()
            .map_err(|_| ColorParseError::InvalidFloat { at: "h" })?;

        if c.is_nan() {
            return Err(ColorParseError::InvalidFloat { at: "c" });
        }
        if h.is_nan() {
            return Err(ColorParseError::InvalidFloat { at: "h" });
        }

        let alpha = if let Some(a_str) = alpha_part {
            let a = a_str
                .parse::<f32>()
                .map_err(|_| ColorParseError::InvalidFloat { at: "a" })?;
            if a.is_nan() {
                return Err(ColorParseError::InvalidFloat { at: "a" });
            }

            Some(a)
        } else {
            None
        };

        Ok(ColorOklch { l, c, h, alpha })
    }
}

impl FromStr for ColorNamed {
    type Err = ColorParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (base, alpha_part) = s
            .split_once(':')
            .map(|(b, a)| (b, Some(a)))
            .unwrap_or((s, None));
        let (name_part, variant_part) = base
            .split_once('-')
            .map(|(n, v)| (n, Some(v)))
            .unwrap_or((base, None));

        let name =
            ColorName::from_str(name_part).map_err(|_| ColorParseError::UnknownColorName {
                name: name_part.to_string(),
            })?;

        let variant = if let Some(v_str) = variant_part {
            let v_u16 = v_str
                .parse::<u16>()
                .map_err(|_| ColorParseError::InvalidVariant {
                    s: v_str.to_string(),
                })?;
            ColorVariant::new(v_u16).map_err(|_| ColorParseError::InvalidVariant {
                s: v_str.to_string(),
            })?
        } else {
            ColorVariant::default()
        };

        let alpha = if let Some(a_str) = alpha_part {
            let a: f32 = a_str
                .parse()
                .map_err(|_| ColorParseError::InvalidFloat { at: "a" })?;
            if a.is_nan() {
                return Err(ColorParseError::InvalidFloat { at: "a" });
            }
            Some(a)
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
        let r = (self.r * 255.0).round() as u8;
        let g = (self.g * 255.0).round() as u8;
        let b = (self.b * 255.0).round() as u8;
        if let Some(a) = self.alpha {
            let a = (a * 255.0).round() as u8;
            write!(f, "#{:02x}{:02x}{:02x}{:02x}", r, g, b, a)
        } else {
            write!(f, "#{:02x}{:02x}{:02x}", r, g, b)
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
            write!(f, ":{:.2}", a)?;
        }
        Ok(())
    }
}

#[cfg(feature = "serde")]
mod _s {
    use std::str::FromStr;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use crate::v1::types::misc::Color;

    impl Serialize for Color {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.serialize_str(&self.to_string())
        }
    }

    impl<'de> Deserialize<'de> for Color {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let s = String::deserialize(deserializer)?;
            Self::from_str(&s).map_err(serde::de::Error::custom)
        }
    }
}
