// TEMP: reexport
pub use common::v1::types::misc::duration::Duration;

// TODO: move these tests to crate-common/src/v1/types/misc/duration.rs
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
