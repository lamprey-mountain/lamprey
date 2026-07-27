use core::fmt;
use std::str::FromStr;

// TODO: validation
// - relative is incompatible with everything
// - only one of t or T may be specified
// - only one of d, D, F may be specified
// - duplicates are not allowed

// TODO: rendering (TODO: apply these as doc comments)
// - this is done in frontend ui, not in rust
// - date means to render something like "mm/dd/yyyy" or "dd/mm/yyyy"
// - long date means to render something like "April 20, 2021"
// - full date means to render something like "Tuesday, April 20, 2021"
// - time renders "hh:mm"
// - time long renders "hh:mm:ss"
// - relative renders "4 years ago"

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateStyle {
    Short, // d
    Long,  // D
    Full,  // F
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeStyle {
    Short, // t
    Long,  // T
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AbsoluteStyle {
    pub date: Option<DateStyle>,
    pub time: Option<TimeStyle>,
}

/// how to render a timestamp
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "wasm",
    derive(tsify::Tsify),
    tsify(into_wasm_abi, type = "string")
)]
pub enum TimestampStyle {
    #[default]
    Relative,
    Absolute(AbsoluteStyle),
}

// TODO: derive error via thiserror
#[derive(Debug, Clone)]
pub enum TimestampStyleParseError {
    UnknownFlag(char),
    DuplicateFlag(char),

    /// eg. `d` then `D`
    ConflictingDate(char, char),

    /// eg. `t` then `T`
    ConflictingTime(char, char),

    /// `r` mixed with anything else
    RelativeCombined(char),
}

// TODO: write tests for this
impl FromStr for TimestampStyle {
    type Err = TimestampStyleParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut relative = false;
        let mut date: Option<(DateStyle, char)> = None;
        let mut time: Option<(TimeStyle, char)> = None;
        let mut seen: Vec<char> = Vec::new();

        for c in s.chars() {
            if seen.contains(&c) {
                return Err(TimestampStyleParseError::DuplicateFlag(c));
            }
            seen.push(c);

            match c {
                'r' => relative = true,
                't' | 'T' => {
                    let style = if c == 't' {
                        TimeStyle::Short
                    } else {
                        TimeStyle::Long
                    };
                    if let Some((_, prev)) = time {
                        return Err(TimestampStyleParseError::ConflictingTime(prev, c));
                    }
                    time = Some((style, c));
                }
                'd' | 'D' | 'F' => {
                    let style = match c {
                        'd' => DateStyle::Short,
                        'D' => DateStyle::Long,
                        _ => DateStyle::Full,
                    };
                    if let Some((_, prev)) = date {
                        return Err(TimestampStyleParseError::ConflictingDate(prev, c));
                    }
                    date = Some((style, c));
                }
                other => return Err(TimestampStyleParseError::UnknownFlag(other)),
            }
        }

        if relative {
            match (date, time) {
                (Some((_, c)), None) | (None, Some((_, c))) => {
                    return Err(TimestampStyleParseError::RelativeCombined(c));
                }
                _ => return Ok(TimestampStyle::Relative),
            }
        }

        Ok(TimestampStyle::Absolute(AbsoluteStyle {
            date: date.map(|(d, _)| d),
            time: time.map(|(t, _)| t),
        }))
    }
}

impl fmt::Display for TimestampStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimestampStyle::Relative => write!(f, "r"),
            TimestampStyle::Absolute(AbsoluteStyle { date, time }) => {
                if let Some(d) = date {
                    let c = match d {
                        DateStyle::Short => 'd',
                        DateStyle::Long => 'D',
                        DateStyle::Full => 'F',
                    };
                    write!(f, "{c}")?;
                }
                if let Some(t) = time {
                    let c = match t {
                        TimeStyle::Short => 't',
                        TimeStyle::Long => 'T',
                    };
                    write!(f, "{c}")?;
                }
                Ok(())
            }
        }
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for TimestampStyle {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
