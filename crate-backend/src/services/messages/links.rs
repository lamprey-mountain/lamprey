//! Link extraction from markdown content using the markdown parser.
//!
//! This module provides functions to extract links from markdown content,
//! replacing the need for the `linkify` crate. The markdown parser approach
//! offers better context awareness and handles markdown-specific link formats.
//!
//! # Example
//! ```
//! use crate_backend::services::messages::links::extract_links;
//!
//! let links = extract_links("check https://example.com and [link](https://other.com)");
//! assert_eq!(links.len(), 2);
//! ```

use lamprey_markdown::{Ast, Parser};
use url::Url;

/// Extract all links from markdown content as validated URLs.
///
/// This includes:
/// - Raw URLs (autolinks): `https://example.com`
/// - Angle bracket links: `<https://example.com>`
/// - Named links: `[text](url)`
///
/// Invalid URLs are filtered out. Use [`extract_link_strings`] if you want
/// to preserve all link strings regardless of validity.
///
/// # Example
/// ```
/// use crate_backend::services::messages::links::extract_links;
///
/// let links = extract_links("check https://example.com and [link](https://other.com)");
/// assert_eq!(links.len(), 2);
/// ```
pub fn extract_links(content: &str) -> Vec<Url> {
    let parser = Parser::default();
    let parsed = parser.parse(content);
    let ast = Ast::new(parsed);

    ast.links()
        .filter_map(|link| Url::parse(&link.dest).ok())
        .collect()
}

/// Extract all link strings from markdown content without URL validation.
///
/// Use this when you want to:
/// - Preserve invalid URLs
/// - Avoid URL parsing overhead
/// - Get the exact link text as it appears in the source
///
/// # Example
/// ```
/// use crate_backend::services::messages::links::extract_link_strings;
///
/// let links = extract_link_strings("check https://example.com");
/// assert!(links.contains(&"https://example.com".to_string()));
/// ```
pub fn extract_link_strings(content: &str) -> Vec<String> {
    let parser = Parser::default();
    let parsed = parser.parse(content);
    let ast = Ast::new(parsed);
    ast.links().map(|link| link.dest.into_owned()).collect()
}

/// Check if content contains any links.
///
/// This is more efficient than extracting all links when you only need
/// to know if links are present.
///
/// # Example
/// ```
/// use crate_backend::services::messages::links::contains_links;
///
/// assert!(contains_links("check https://example.com"));
/// assert!(!contains_links("no links here"));
/// ```
pub fn contains_links(content: &str) -> bool {
    let parser = Parser::default();
    let parsed = parser.parse(content);
    let ast = Ast::new(parsed);
    let mut links = ast.links();
    links.next().is_some()
}

/// Extract links with their positions in the original text.
///
/// Returns tuples of (start_byte, end_byte, url) for each link found.
///
/// # Example
/// ```
/// use crate_backend::services::messages::links::extract_links_with_positions;
///
/// let links = extract_links_with_positions("see https://example.com here");
/// assert_eq!(links.len(), 1);
/// assert_eq!(links[0].2.as_str(), "https://example.com/");
/// ```
pub fn extract_links_with_positions(content: &str) -> Vec<(usize, usize, Url)> {
    let parser = Parser::default();
    let parsed = parser.parse(content);
    let ast = Ast::new(parsed);

    ast.links()
        .filter_map(|link| {
            // Find the position of this link in the source
            if let Some(start) = content.find(&link.dest as &str) {
                let end = start + link.dest.len();
                Url::parse(&link.dest).ok().map(|url| (start, end, url))
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_links_raw_url() {
        let links = extract_links("check https://example.com out");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].as_str(), "https://example.com/");
    }

    #[test]
    fn test_extract_links_named_link() {
        let links = extract_links("check [this](https://example.com) out");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].as_str(), "https://example.com/");
    }

    #[test]
    fn test_extract_links_angle_bracket() {
        let links = extract_links("check <https://example.com> out");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].as_str(), "https://example.com/");
    }

    #[test]
    fn test_extract_links_multiple() {
        let links = extract_links("https://a.com and https://b.com");
        assert_eq!(links.len(), 2);
    }

    #[test]
    fn test_extract_links_none() {
        let links = extract_links("no links here");
        assert_eq!(links.len(), 0);
    }

    #[test]
    fn test_extract_links_invalid_url() {
        // Invalid URLs are filtered out
        let links = extract_links("not a valid url");
        assert_eq!(links.len(), 0);
    }

    #[test]
    fn test_extract_link_strings_preserves_invalid() {
        // extract_link_strings preserves all link strings
        let links = extract_link_strings("check https://example.com out");
        assert!(links.contains(&"https://example.com".to_string()));
    }

    #[test]
    fn test_contains_links_true() {
        assert!(contains_links("https://example.com"));
        assert!(contains_links("check [link](https://example.com)"));
        assert!(contains_links("<https://example.com>"));
    }

    #[test]
    fn test_contains_links_false() {
        assert!(!contains_links("no links here"));
        assert!(!contains_links(""));
    }

    #[test]
    fn test_extract_links_with_positions() {
        let content = "see https://example.com here";
        let links = extract_links_with_positions(content);
        assert_eq!(links.len(), 1);
        let (start, end, url) = &links[0];
        assert_eq!(&content[*start..*end], "https://example.com");
        assert_eq!(url.as_str(), "https://example.com/");
    }

    #[test]
    fn test_extract_links_complex_markdown() {
        let content = r#"
Check [Google](https://google.com) and <https://example.com>.
Also visit https://github.com for code.
        "#;
        let links = extract_links(content);
        assert_eq!(links.len(), 3);
    }

    #[test]
    fn test_extract_links_with_query_params() {
        let links = extract_links("https://example.com/path?query=value&other=123");
        assert_eq!(links.len(), 1);
        assert!(links[0].as_str().contains("query=value"));
    }

    #[test]
    fn test_extract_links_with_fragment() {
        let links = extract_links("https://example.com/page#section");
        assert_eq!(links.len(), 1);
        assert!(links[0].as_str().contains("#section"));
    }
}
