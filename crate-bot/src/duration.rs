use std::str::FromStr;

use anyhow::{anyhow, bail};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Duration with support for compound times like "5m30s", "1h2m30s"
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Duration {
    seconds: i64,
}

impl Duration {
    pub fn seconds(&self) -> i64 {
        self.seconds
    }
}

impl FromStr for Duration {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut total_seconds = 0i64;
        let mut remaining = s;

        loop {
            if remaining.is_empty() {
                break;
            }

            // Find the start of the number (skip any non-digit chars)
            let start = remaining
                .find(|c: char| c.is_ascii_digit() || (c == '-' && remaining.len() > 1))
                .ok_or_else(|| anyhow!("invalid duration format: missing number"))?;

            if start > 0 {
                remaining = &remaining[start..];
            }

            // Find the end of the number
            let end = remaining
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(remaining.len());

            if end == 0 {
                bail!("invalid duration format: empty number");
            }

            let num_str = &remaining[..end];
            let num: i64 = num_str.parse()?;

            remaining = &remaining[end..];

            // Parse the unit
            let unit = remaining
                .chars()
                .take_while(|c| c.is_alphabetic())
                .collect::<String>();
            remaining = &remaining[unit.len()..];

            if unit.is_empty() {
                bail!("invalid duration format: missing unit");
            }

            let seconds = match unit.as_str() {
                "s" | "sec" | "second" | "seconds" => num,
                "m" | "min" | "minute" | "minutes" => num * 60,
                "h" | "hr" | "hour" | "hours" => num * 3600,
                "d" | "day" | "days" => num * 86400,
                "w" | "week" | "weeks" => num * 604800,
                _ => bail!("unknown duration unit: {}", unit),
            };

            total_seconds += seconds;
        }

        if total_seconds <= 0 {
            bail!("duration must be positive");
        }

        Ok(Duration {
            seconds: total_seconds,
        })
    }
}

impl Serialize for Duration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Duration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Duration::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for Duration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut seconds = self.seconds;

        let weeks = seconds / 604800;
        seconds %= 604800;

        let days = seconds / 86400;
        seconds %= 86400;

        let hours = seconds / 3600;
        seconds %= 3600;

        let minutes = seconds / 60;
        seconds %= 60;

        write!(
            f,
            "{}",
            if weeks > 0 {
                format!("{}w", weeks)
            } else {
                "".to_string()
            }
        )?;
        write!(
            f,
            "{}",
            if days > 0 {
                format!("{}d", days)
            } else {
                "".to_string()
            }
        )?;
        write!(
            f,
            "{}",
            if hours > 0 {
                format!("{}h", hours)
            } else {
                "".to_string()
            }
        )?;
        write!(
            f,
            "{}",
            if minutes > 0 {
                format!("{}m", minutes)
            } else {
                "".to_string()
            }
        )?;
        write!(f, "{}s", seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_durations() {
        assert_eq!(Duration::from_str("5s").unwrap().seconds(), 5);
        assert_eq!(Duration::from_str("5m").unwrap().seconds(), 300);
        assert_eq!(Duration::from_str("1h").unwrap().seconds(), 3600);
        assert_eq!(Duration::from_str("1d").unwrap().seconds(), 86400);
        assert_eq!(Duration::from_str("1w").unwrap().seconds(), 604800);
    }

    #[test]
    fn test_compound_durations() {
        assert_eq!(Duration::from_str("5m30s").unwrap().seconds(), 330);
        assert_eq!(Duration::from_str("1h2m30s").unwrap().seconds(), 3750);
        assert_eq!(Duration::from_str("2d3h").unwrap().seconds(), 183600);
        assert_eq!(Duration::from_str("1w2d").unwrap().seconds(), 777600);
    }

    #[test]
    fn test_duration_aliases() {
        assert_eq!(Duration::from_str("5sec").unwrap().seconds(), 5);
        assert_eq!(Duration::from_str("5second").unwrap().seconds(), 5);
        assert_eq!(Duration::from_str("5seconds").unwrap().seconds(), 5);

        assert_eq!(Duration::from_str("5min").unwrap().seconds(), 300);
        assert_eq!(Duration::from_str("5minute").unwrap().seconds(), 300);
        assert_eq!(Duration::from_str("5minutes").unwrap().seconds(), 300);

        assert_eq!(Duration::from_str("1hr").unwrap().seconds(), 3600);
        assert_eq!(Duration::from_str("1hour").unwrap().seconds(), 3600);
        assert_eq!(Duration::from_str("1hours").unwrap().seconds(), 3600);

        assert_eq!(Duration::from_str("1day").unwrap().seconds(), 86400);
        assert_eq!(Duration::from_str("1days").unwrap().seconds(), 86400);

        assert_eq!(Duration::from_str("1week").unwrap().seconds(), 604800);
        assert_eq!(Duration::from_str("1weeks").unwrap().seconds(), 604800);
    }

    #[test]
    fn test_complex_compound_durations() {
        assert_eq!(
            Duration::from_str("1w2d3h4m5s").unwrap().seconds(),
            604800 + 172800 + 10800 + 240 + 5
        );
        assert_eq!(Duration::from_str("2h30m15s").unwrap().seconds(), 9015);
        assert_eq!(Duration::from_str("3d12h").unwrap().seconds(), 302400);
    }

    #[test]
    fn test_display_format() {
        assert_eq!(Duration::from_str("5s").unwrap().to_string(), "5s");
        assert_eq!(Duration::from_str("65s").unwrap().to_string(), "1m5s");
        assert_eq!(Duration::from_str("3665s").unwrap().to_string(), "1h1m5s");
        assert_eq!(
            Duration::from_str("90065s").unwrap().to_string(),
            "1d1h1m5s"
        );
        assert_eq!(
            Duration::from_str("694865s").unwrap().to_string(),
            "1w1d1h1m5s"
        );
    }

    #[test]
    fn test_invalid_durations() {
        assert!(Duration::from_str("").is_err());
        assert!(Duration::from_str("s").is_err());
        assert!(Duration::from_str("5").is_err());
        assert!(Duration::from_str("5x").is_err());
        assert!(Duration::from_str("-5s").is_err());
        assert!(Duration::from_str("0s").is_err());
    }

    #[test]
    fn test_zero_duration() {
        assert!(Duration::from_str("0s").is_err());
        assert!(Duration::from_str("0m").is_err());
        assert!(Duration::from_str("0h").is_err());
    }

    #[test]
    fn test_large_duration() {
        let duration = Duration::from_str("100w50d20h10m30s").unwrap();
        let expected = 100 * 604800 + 50 * 86400 + 20 * 3600 + 10 * 60 + 30;
        assert_eq!(duration.seconds(), expected);
    }

    #[test]
    fn test_partial_compound_with_gaps() {
        assert_eq!(Duration::from_str("1h5s").unwrap().seconds(), 3605);
        assert_eq!(Duration::from_str("1d10m").unwrap().seconds(), 87000);
        assert_eq!(Duration::from_str("2w3s").unwrap().seconds(), 1209603);
    }
}
