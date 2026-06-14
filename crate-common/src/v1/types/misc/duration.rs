use core::{
    fmt,
    ops::{Add, Sub},
    time::Duration as StdDuration,
};
use std::str::FromStr;

#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

// TODO: swap all duration types to this

/// a duration, aka a span of time
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "utoipa", derive(ToSchema), schema(value_type = String))]
pub struct Duration(u64);

impl Duration {
    /// create a new duration as a specified number of milliseconds
    pub fn from_millis(ms: u64) -> Self {
        Self(ms)
    }

    /// get the total number of milliseconds in this duration
    pub fn as_millis(&self) -> u64 {
        self.0
    }
}

#[cfg(feature = "serde")]
impl Serialize for Duration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Duration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Helper {
            String(String),
            Number(u64),
        }

        match Helper::deserialize(deserializer)? {
            Helper::String(s) => s
                .parse::<Duration>()
                .map_err(|_| serde::de::Error::custom("invalid duration string")),
            Helper::Number(n) => Ok(Duration::from_millis(n)),
        }
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ms = self.0;
        let w = ms / (1000 * 60 * 60 * 24 * 7);
        ms %= 1000 * 60 * 60 * 24 * 7;
        let d = ms / (1000 * 60 * 60 * 24);
        ms %= 1000 * 60 * 60 * 24;
        let h = ms / (1000 * 60 * 60);
        ms %= 1000 * 60 * 60;
        let m = ms / (1000 * 60);
        ms %= 1000 * 60;
        let s = ms / 1000;
        ms %= 1000;

        // PERF: stream to writer instead of collecting in a vec
        let mut parts = Vec::new();
        if w > 0 {
            parts.push(format!("{}w", w));
        }
        if d > 0 {
            parts.push(format!("{}d", d));
        }
        if h > 0 {
            parts.push(format!("{}h", h));
        }
        if m > 0 {
            parts.push(format!("{}m", m));
        }
        if s > 0 {
            parts.push(format!("{}s", s));
        }
        if ms > 0 || parts.is_empty() {
            parts.push(format!("{}ms", ms));
        }

        write!(f, "{}", parts.join(" "))
    }
}

impl FromStr for Duration {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        // bare number = milliseconds
        if let Ok(ms) = s.parse::<u64>() {
            return Ok(Self(ms));
        }

        let mut total_ms = 0u64;
        let mut current_num = String::with_capacity(12);
        let mut current_unit = String::with_capacity(12);

        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if c.is_ascii_digit() {
                current_num.push(c);
            } else if c.is_alphabetic() {
                // read unit
                while i < chars.len() && chars[i].is_alphabetic() {
                    current_unit.push(chars[i]);
                    i += 1;
                }
                i -= 1; // backstep because loop will increment

                let num = current_num.parse::<u64>().map_err(|_| ())?;

                let multiplier = match current_unit.to_lowercase().as_str() {
                    "ms" | "millis" | "millisecond" | "milliseconds" => 1,
                    "s" | "sec" | "secs" | "second" | "seconds" => 1000,
                    "m" | "min" | "mins" | "minute" | "minutes" => 1000 * 60,
                    "h" | "hr" | "hrs" | "hour" | "hours" => 1000 * 60 * 60,
                    "d" | "day" | "days" => 1000 * 60 * 60 * 24,
                    "w" | "wk" | "week" | "weeks" => 1000 * 60 * 60 * 24 * 7,
                    _ => return Err(()),
                };
                total_ms += num * multiplier;

                current_num.clear();
                current_unit.clear();
            }
            i += 1;
        }

        Ok(Self(total_ms))
    }
}

impl From<StdDuration> for Duration {
    fn from(d: StdDuration) -> Self {
        Self(d.as_millis() as u64)
    }
}

impl From<Duration> for StdDuration {
    fn from(d: Duration) -> Self {
        StdDuration::from_millis(d.as_millis())
    }
}

impl Add<Duration> for Duration {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl Sub<Duration> for Duration {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_parse(s: &str, expected: u64) {
        let d = Duration::from_str(s).unwrap();
        assert_eq!(d.as_millis(), expected, "failed to parse {}", s);
    }

    #[test]
    fn test_parse() {
        assert_parse("12345", 12345);
        assert_parse("12h34s56ms", 43234056);
        assert_parse("  12h    34s     56ms    ", 43234056);
        assert_parse("12hr 34sec 56millis", 43234056);
        assert_parse("12 hours 34 seconds 56 milliseconds", 43234056);
    }

    #[test]
    fn test_display() {
        assert_eq!(Duration::from_millis(43234056).to_string(), "12h 34s 56ms");
        assert_eq!(Duration::from_millis(12345).to_string(), "12s 345ms");
    }

    #[test]
    fn test_ops() {
        let d1 = Duration::from_millis(100);
        let d2 = Duration::from_millis(200);
        assert_eq!((d1 + d2).as_millis(), 300);
        assert_eq!((d2 - d1).as_millis(), 100);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde() {
        let d = Duration::from_millis(43234056);
        let s = serde_json::to_string(&d).unwrap();
        assert_eq!(s, "\"12h 34s 56ms\"");

        let d2: Duration = serde_json::from_str(&s).unwrap();
        assert_eq!(d, d2);

        let d3: Duration = serde_json::from_str("12345").unwrap();
        assert_eq!(d3.as_millis(), 12345);
    }
}
