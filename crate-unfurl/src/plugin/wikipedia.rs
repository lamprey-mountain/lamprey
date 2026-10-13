use async_trait::async_trait;
use lamprey_common::v1::types::EmbedType;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde::Deserialize;
use url::Url;

use crate::{
    Unfurler,
    plugin::{Plugin, PluginUrl, UnfurlResult},
    unfurler::{EmbedGeneration, UnfurlerBuilder},
    util::{EmbedGenerationTemplate, EmbedMediaPending},
};

/// a plugin which handles wikipedia articles
#[derive(Debug, Default)]
pub struct WikipediaPlugin {
    _nope: (), // prevent users from manually constructing
}

impl Plugin for WikipediaPlugin {
    fn register(self, builder: UnfurlerBuilder) -> UnfurlerBuilder {
        builder.add_plugin_url(self)
    }
}

#[async_trait]
impl PluginUrl for WikipediaPlugin {
    async fn handle(&self, unfurler: &Unfurler, url: &Url) -> UnfurlResult {
        let Some(summary_url) = wikipedia_summary_url(url) else {
            return UnfurlResult::skip();
        };

        let res = unfurler.http_client().get(summary_url).send().await;

        let response = match res {
            Ok(r) => r,
            Err(e) => return UnfurlResult::error(e.into()),
        };

        let response = match response.error_for_status() {
            Ok(r) => r,
            Err(e) => return UnfurlResult::error(e.into()),
        };

        let summary: Summary = match response.json().await {
            Ok(s) => s,
            Err(e) => return UnfurlResult::error(e.into()),
        };

        let thumbnail = if let Some(i) = summary.thumbnail
            && summary.originalimage.is_none()
        {
            Some(EmbedMediaPending::new(i.source).into())
        } else {
            None
        };

        let media = if let Some(i) = summary.originalimage {
            Some(EmbedMediaPending::new(i.source).into())
        } else {
            None
        };

        let embed = EmbedGeneration {
            embed: EmbedGenerationTemplate {
                ty: EmbedType::Link,
                url: Some(url.clone()),
                canonical_url: None,
                title: Some(summary.titles.normalized),
                description: Some(summary.extract),
                color: None,
                media,
                thumbnail,
                author_name: None,
                author_url: None,
                author_avatar: None,
                site_name: Some("Wikipedia".to_string()),
                site_avatar: None,
            },
        };

        UnfurlResult {
            embeds: vec![embed],
            errors: vec![],
            stop: false,
        }
    }
}

fn wikipedia_summary_url(url: &Url) -> Option<Url> {
    let host = url.host_str()?;
    if !host.ends_with(".wikipedia.org") && host != "wikipedia.org" {
        return None;
    }

    // "en.wikipedia.org" -> "en", "en.m.wikipedia.org" -> "en"
    let lang = host
        .strip_suffix(".wikipedia.org")?
        .split('.')
        .next()?
        .to_string();
    if lang.is_empty() {
        return None; // bare "wikipedia.org"
    }

    let title = extract_title(url)?;
    let encoded_title = utf8_percent_encode(&title, NON_ALPHANUMERIC).to_string();

    Url::parse(&format!(
        "https://{lang}.wikipedia.org/api/rest_v1/page/summary/{encoded_title}"
    ))
    .ok()
}

fn extract_title(url: &Url) -> Option<String> {
    let mut segments = url.path_segments()?;

    match segments.next()? {
        // https://en.wikipedia.org/wiki/Rust_(programming_language)
        "wiki" => segments
            .next()
            .filter(|s| !s.is_empty())
            .map(|s| percent_decode(s)),
        // https://en.wikipedia.org/w/index.php?title=Rust_(programming_language)
        "w" => url
            .query_pairs()
            .find(|(k, _)| k == "title")
            .map(|(_, v)| v.into_owned()),
        _ => None,
    }
}

fn percent_decode(s: &str) -> String {
    percent_encoding::percent_decode_str(s)
        .decode_utf8_lossy()
        .into_owned()
}

#[derive(Debug, Deserialize)]
struct Summary {
    titles: Titles,
    // description: Option<String>,
    extract: String,
    // lang: String,
    originalimage: Option<Image>,
    thumbnail: Option<Image>,
    // coordinates: Coordinates,
}

#[derive(Debug, Deserialize)]
struct Titles {
    // canonical: String,
    normalized: String,
    // display: String,
}

#[derive(Debug, Deserialize)]
struct Image {
    source: Url,
    // width: u64,
    // height: u64,
}

// // TODO(future): use this somehow?
// #[derive(Debug, Deserialize)]
// struct Coordinates {
//     lat: f64,
//     lon: f64,
// }
