use std::io::Write;
use std::sync::Arc;
use std::time::Duration;

use axum::response::IntoResponse;
use axum::{extract::State, Json};
use mediatype::MediaType;
use serde::Deserialize;
use tracing::info;
use types::{MediaId, UrlEmbed, UrlEmbedRequest};
use url::Url;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::ServerState;

use super::util::Auth;
use crate::error::{Error, Result};

use webpage::HTML;

const USER_AGENT: &str = "StupidTestBot (no url yet)";

const MAX_SIZE_HTML: u64 = 1024 * 1024 * 1;
const MAX_SIZE_ATTACHMENT: u64 = 1024 * 1024 * 8;

// https://ogp.me/#types
#[derive(Debug, Deserialize)]
enum OpenGraphType {
    #[serde(rename = "music.song")]
    MusicSong,
    #[serde(rename = "music.album")]
    MusicAlbum,
    #[serde(rename = "music.playlist")]
    MusicPlaylist,
    #[serde(rename = "music.radio_station")]
    MusicRadioStation,
    #[serde(rename = "video.movie")]
    VideoMovie,
    #[serde(rename = "video.episode")]
    VideoEpisode,
    #[serde(rename = "video.other")]
    VideoOther,
    #[serde(rename = "article")]
    Article,
    #[serde(rename = "book")]
    Book,
    #[serde(rename = "profile")]
    Profile,
    #[serde(rename = "website")]
    Website,
    #[serde(other)]
    Other,
}

impl OpenGraphType {
    pub fn is_media_probably_thumbnail(&self) -> bool {
        match self {
            OpenGraphType::MusicSong => true,
            OpenGraphType::MusicAlbum => true,
            OpenGraphType::MusicPlaylist => true,
            OpenGraphType::MusicRadioStation => true,
            OpenGraphType::VideoMovie => false,
            OpenGraphType::VideoEpisode => false,
            OpenGraphType::VideoOther => false,
            OpenGraphType::Article => true,
            OpenGraphType::Book => true,
            OpenGraphType::Profile => true,
            OpenGraphType::Website => true,
            OpenGraphType::Other => true,
        }
    }
}

struct ParsedMedia {
    url: Url,
    alt: Option<String>,
}

fn get_media(parsed: &HTML) -> Option<ParsedMedia> {
    for vid in &parsed.opengraph.videos {
        let c: Option<MediaType> = vid
            .properties
            .get("content-type")
            .and_then(|s| MediaType::parse(s).ok());
        if c.is_none_or(|c| c.ty == "video") {
            return Some(ParsedMedia {
                url: vid.url.parse().ok()?,
                alt: vid.properties.get("alt").map(|s| s.to_owned()),
            });
        }
    }

    for img in &parsed.opengraph.images {
        let c: Option<MediaType> = img
            .properties
            .get("content-type")
            .and_then(|s| MediaType::parse(s).ok());
        if c.is_none_or(|c| c.ty == "image") {
            return Some(ParsedMedia {
                url: img.url.parse().ok()?,
                alt: img.properties.get("alt").map(|s| s.to_owned()),
            });
        }
    }

    for aud in &parsed.opengraph.audios {
        let c: Option<MediaType> = aud
            .properties
            .get("content-type")
            .and_then(|s| MediaType::parse(s).ok());
        if c.is_none_or(|c| c.ty == "audio") {
            return Some(ParsedMedia {
                url: aud.url.parse().ok()?,
                alt: aud.properties.get("alt").map(|s| s.to_owned()),
            });
        }
    }

    None
}

/// Embed a url
#[utoipa::path(
    post,
    path = "/debug/embed-url",
    tags = ["debug"],
    responses(
        (status = OK, body = UrlEmbed, description = "success"),
    )
)]
pub async fn debug_embed_url(
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<UrlEmbedRequest>,
) -> Result<impl IntoResponse> {
    info!("generating url embed user_id={user_id} url={}", json.url);
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .connect_timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::limited(10))
        .user_agent(USER_AGENT)
        .https_only(true)
        .build()?;
    let mut fetched = http
        .get(json.url.clone())
        .timeout(Duration::from_secs(15))
        .send()
        .await?
        .error_for_status()?;
    let content_length = fetched.content_length();
    let content_type = fetched.headers().get("content-type");
    // TODO: try to parse name from Content-Disposition
    let media_id = MediaId(Uuid::now_v7());
    let srv = s.services();
    if content_type.is_some_and(|c| c == "a") {
        let canonical_url = fetched.url().to_owned();
        let filename = json
            .url
            .path_segments()
            .and_then(|p| p.last())
            .map(|s| s.to_owned())
            .unwrap_or_else(|| "index.html".to_owned());
        let media = srv
            .media
            .import_from_response(
                user_id,
                media_id,
                types::MediaCreate {
                    alt: None,
                    source: types::MediaCreateSource::Download {
                        filename: Some(filename),
                        size: content_length,
                        source_url: json.url.clone(),
                    },
                },
                fetched,
                MAX_SIZE_ATTACHMENT,
            )
            .await?;
        let embed = UrlEmbed {
            url: json.url.clone(),
            canonical_url: if json.url == canonical_url {
                None
            } else {
                Some(canonical_url)
            },
            title: None,
            description: None,
            color: None,
            media: Some(media),
            media_is_thumbnail: false,
            author_url: None,
            author_name: None,
            author_avatar: None,
            site_name: None,
            site_avatar: None,
        };
        Ok(Json(dbg!(embed)))
    } else {
        if content_length.is_some_and(|c| c > MAX_SIZE_HTML) {
            return Err(Error::TooBig);
        }

        let mut buf =
            Vec::with_capacity(content_length.unwrap_or(MAX_SIZE_HTML).try_into().unwrap());
        while let Some(chunk) = fetched.chunk().await? {
            buf.write_all(&chunk)?;
            if buf.len() as u64 > MAX_SIZE_HTML {
                return Err(Error::TooBig);
            }
            if content_length.is_some_and(|c| buf.len() as u64 > c) {
                return Err(Error::TooBig);
            }
        }

        let html = String::from_utf8_lossy(&buf);
        let parsed = HTML::from_string(html.into_owned(), Some(json.url.to_string()))
            .map_err(Error::UrlEmbed)?;
        dbg!(&parsed);
        let canonical_url = parsed
            .url
            .as_ref()
            .map(|u| u.parse())
            .transpose()?
            .unwrap_or(fetched.url().to_owned());
        let title = parsed
            .opengraph
            .properties
            .get("title")
            .or(parsed.title.as_ref())
            .map(ToOwned::to_owned);
        let description = parsed
            .opengraph
            .properties
            .get("description")
            .or(parsed.description.as_ref())
            .map(ToOwned::to_owned);
        let site_name = parsed
            .opengraph
            .properties
            .get("site_name")
            .map(ToOwned::to_owned);
        let m = get_media(&parsed);
        let og_type: OpenGraphType =
            serde_json::from_value(serde_json::Value::String(parsed.opengraph.og_type))?;

        #[derive(Debug, PartialEq)]
        enum MediaInstructions {
            Thumb,
            Full,
            Hide,
        }

        let media_type = match parsed.meta.get("twitter:card").map(|s| s.as_str()) {
            Some("summary_large_image") => MediaInstructions::Full,
            Some(_) => MediaInstructions::Thumb,
            None => {
                let robots_instructions: Vec<&str> = parsed
                    .meta
                    .get("robots")
                    .map(|s| s.split(",").map(|s| s.trim()).collect())
                    .unwrap_or_default();
                // also: nosnippet, max-snippet:100, max-video-preview:100
                if robots_instructions.contains(&"max-image-preview:none") {
                    MediaInstructions::Hide
                } else if robots_instructions.contains(&"max-image-preview:standard") {
                    MediaInstructions::Full
                } else if robots_instructions.contains(&"max-image-preview:large") {
                    MediaInstructions::Thumb
                } else if og_type.is_media_probably_thumbnail() {
                    MediaInstructions::Thumb
                } else {
                    MediaInstructions::Full
                }
            }
        };

        let media = if let Some(m) = m {
            Some(
                srv.media
                    .import_from_url_with_max_size(
                        user_id,
                        types::MediaCreate {
                            alt: m.alt,
                            source: types::MediaCreateSource::Download {
                                filename: None,
                                size: None,
                                source_url: m.url,
                            },
                        },
                        MAX_SIZE_ATTACHMENT,
                    )
                    .await?,
            )
        } else {
            None
        };

        let embed = UrlEmbed {
            url: json.url.clone(),
            canonical_url: if json.url == canonical_url {
                None
            } else {
                Some(canonical_url)
            },
            title,
            description,
            // TODO: parse meta.get("theme-color");
            color: None,
            media,
            media_is_thumbnail: match media_type {
                MediaInstructions::Thumb => true,
                MediaInstructions::Full => false,
                MediaInstructions::Hide => false,
            },
            // TODO: parse author information
            author_url: None,
            author_name: None,
            author_avatar: None,
            site_name,
            // TODO: fetch favicon
            site_avatar: None,
        };
        Ok(Json(dbg!(embed)))
    }
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new().routes(routes!(debug_embed_url))
}
