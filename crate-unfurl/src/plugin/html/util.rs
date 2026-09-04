use std::str::FromStr;

/// Robots max-image-preview directive
// TODO: parse X-Robots-Tag HTTP header
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RobotsImagePreview {
    /// don't render image at all
    None,

    /// render image as a thumbnail
    Standard,

    /// render image as media
    Large,
}

/// Twitter card type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TwitterCard {
    Summary,
    SummaryLargeImage,
    Player,
    App,
}

// TODO: use strum for this
impl FromStr for TwitterCard {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "summary" => Ok(TwitterCard::Summary),
            "summary_large_image" => Ok(TwitterCard::SummaryLargeImage),
            "player" => Ok(TwitterCard::Player),
            "app" => Ok(TwitterCard::App),
            _ => Err(()),
        }
    }
}
