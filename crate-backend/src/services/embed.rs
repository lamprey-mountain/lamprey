use std::io::Write;
use std::str::FromStr;
use std::{sync::Arc, time::Duration};

use common::v1::types::misc::Color;
use common::v1::types::{self, MessageSync, MessageType};
use common::v1::types::{Embed, EmbedId};
use common::v1::types::{Media, UserId};
use mediatype::{MediaType, MediaTypeBuf};
use moka::future::Cache;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, Mutex};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};
use url::Url;
use webpage::HTML;

use crate::error::Error;
use crate::types::{DbMessageCreate, MediaLinkType, MessageRef};
use crate::Result;
use crate::ServerStateInner;

const MAX_SIZE_HTML: u64 = 1024 * 1024 * 4;
const MAX_SIZE_ATTACHMENT: u64 = 1024 * 1024 * 8;
const MAX_EMBED_AGE: Duration = Duration::from_secs(60 * 5);

pub struct ServiceEmbed {
    state: Arc<ServerStateInner>,
    cache: Cache<Url, Embed>,
    stop: broadcast::Sender<()>,
    workers: Mutex<Vec<JoinHandle<()>>>,
}
/// an opengraph type
///
/// <https://ogp.me/#types>
#[derive(Debug, PartialEq)]
pub enum OpenGraphType {
    MusicSong,
    MusicAlbum,
    MusicPlaylist,
    MusicRadioStation,
    VideoMovie,
    VideoEpisode,
    VideoOther,
    Article,
    Book,
    Profile,
    Website,
    Object,
    Other,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EmbedType {
    /// a generic website embed
    Website(Embed),

    /// a piece of media
    Media(Media),
    // /// a custom embed
    // Custom(UrlEmbed),
}

/// how to display the attached image
#[derive(Debug, PartialEq)]
enum ImageInstructions {
    /// the image should be displayed as a thumbnail
    Thumb,

    /// the image should be displayed as the main content
    Full,

    /// the image should be ignored
    Hide,
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
            OpenGraphType::Object => false,
            OpenGraphType::Other => false,
        }
    }
}

impl From<&str> for OpenGraphType {
    fn from(value: &str) -> Self {
        // NOTE: some of these aren't standard, but are used in the wild
        match value {
            "music.song" | "music" => Self::MusicSong,
            "music.album" => Self::MusicAlbum,
            "music.playlist" => Self::MusicPlaylist,
            "music.radio_station" => Self::MusicRadioStation,
            "video.movie" => Self::VideoMovie,
            "video.episode" => Self::VideoEpisode,
            "video.other" | "video" => Self::VideoOther,
            "article" => Self::Article,
            "book" => Self::Book,
            "profile" => Self::Profile,
            "website" => Self::Website,
            "object" => Self::Object,
            _ => Self::Other,
        }
    }
}

impl From<OpenGraphType> for &'static str {
    fn from(value: OpenGraphType) -> &'static str {
        match value {
            OpenGraphType::MusicSong => "music.song",
            OpenGraphType::MusicAlbum => "music.album",
            OpenGraphType::MusicPlaylist => "music.playlist",
            OpenGraphType::MusicRadioStation => "music.radio.station",
            OpenGraphType::VideoMovie => "video.movie",
            OpenGraphType::VideoEpisode => "video.episode",
            OpenGraphType::VideoOther => "video.other",
            OpenGraphType::Article => "article",
            OpenGraphType::Book => "book",
            OpenGraphType::Profile => "profile",
            OpenGraphType::Website => "website",
            OpenGraphType::Object => "object",
            OpenGraphType::Other => "other",
        }
    }
}

impl ServiceEmbed {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        let (tx, _) = broadcast::channel(1);
        Self {
            state,
            cache: Cache::builder()
                .max_capacity(1000)
                .time_to_live(MAX_EMBED_AGE)
                .build(),
            stop: tx,
            workers: Mutex::new(Vec::new()),
        }
    }

    pub async fn start_workers(&self) {
        let mut workers_guard = self.workers.lock().await;
        if !workers_guard.is_empty() {
            warn!("embed workers already started");
            return;
        }
        for i in 0..self.state.config.url_preview.max_parallel_jobs {
            let state = self.state.clone();
            let mut stop = self.stop.subscribe();
            workers_guard.push(tokio::spawn(async move {
                info!("starting embed worker {i}");
                loop {
                    tokio::select! {
                        _ = stop.recv() => {
                            info!("stopping embed worker {i}");
                            break;
                        }
                        _ = tokio::time::sleep(Duration::from_secs(1)) => {}
                    }
                    if let Err(e) = Self::worker(state.clone()).await {
                        error!("embed worker failed: {e:?}");
                    }
                }
            }));
        }
    }

    pub async fn stop(self) {
        if self.stop.send(()).is_err() {
            warn!("no embed workers to stop");
        }
        let workers = self.workers.into_inner();
        for worker in workers {
            if let Err(e) = worker.await {
                error!("failed to stop embed worker: {e:?}");
            }
        }
    }

    async fn worker(state: Arc<ServerStateInner>) -> Result<()> {
        let data = state.data();
        let Some(job) = data.url_embed_queue_claim().await? else {
            return Ok(());
        };

        let url: Url = job.url.parse()?;

        let embed = match state
            .services()
            .embed
            .cache
            .try_get_with(url.clone(), async {
                debug!("generating embed for {}", url);
                Self::generate_inner(&state, job.user_id.into(), url)
                    .await
                    .map_err(Arc::new)
            })
            .await
        {
            Ok(embed) => embed,
            Err(e_arc) => {
                if let Err(e) = data.url_embed_queue_finish(job.id, None).await {
                    error!("failed to finish url embed queue job with error: {e:?}");
                }
                return Err(e_arc.fake_clone());
            }
        };

        if let Err(e) = data.url_embed_queue_finish(job.id, Some(&embed)).await {
            error!("failed to finish url embed queue job: {e:?}");
        }
        if let Err(e) = Self::attach_embed(
            &state,
            job.message_ref.map(|v| serde_json::from_value(v).unwrap()),
            job.user_id.into(),
            embed,
        )
        .await
        {
            error!("failed to attach embed: {e:?}");
        }
        Ok(())
    }

    pub async fn queue(
        &self,
        message_ref: Option<MessageRef>,
        user_id: UserId,
        url: Url,
    ) -> Result<()> {
        if let Some(embed) = self.cache.get(&url).await {
            if let Some(message_ref) = message_ref {
                info!(
                    "reuse embed message: version_id = {} url = {:?}",
                    message_ref.version_id,
                    url.as_str()
                );
                if let Err(e) =
                    Self::attach_embed(&self.state, Some(message_ref), user_id, embed).await
                {
                    error!("failed to attach embed from cache: {e:?}");
                }
            }
            return Ok(());
        }

        self.state
            .data()
            .url_embed_queue_insert(message_ref, user_id, url.to_string())
            .await?;
        Ok(())
    }

    #[tracing::instrument(level = "info", skip(state))]
    pub(crate) async fn generate_inner(
        state: &Arc<ServerStateInner>,
        user_id: UserId,
        url: Url,
    ) -> Result<Embed> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::limited(10))
            .user_agent(&state.config.url_preview.user_agent)
            .https_only(true)
            .build()?;
        let fetched = http
            .get(url.clone())
            .timeout(Duration::from_secs(15))
            .send()
            .await?;
        let addr = fetched
            .remote_addr()
            .ok_or(Error::BadStatic("request has no remote ip address"))?;
        for denied in &state.config.url_preview.deny {
            if denied.contains(&addr.ip()) {
                return Err(Error::BadStatic("url blacklisted"));
            }
        }
        let mut fetched = fetched.error_for_status()?;
        let content_length = fetched.content_length();
        let content_type = fetched
            .headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| MediaTypeBuf::from_str(s).ok());
        // TODO: try to parse name from Content-Disposition
        let srv = state.services();
        let embed = if content_type.is_some_and(is_media) {
            debug!("got media");
            let canonical_url = fetched.url().to_owned();
            let filename = url
                .path_segments()
                .and_then(|p| p.last())
                .map(|s| s.to_owned())
                .unwrap_or_else(|| "index.html".to_owned());
            let media = srv
                .media
                .import_from_response(
                    user_id,
                    types::MediaCreate {
                        alt: None,
                        source: types::MediaCreateSource::Download {
                            filename: Some(filename),
                            size: content_length,
                            source_url: url.clone(),
                        },
                    },
                    fetched,
                    MAX_SIZE_ATTACHMENT,
                )
                .await?;
            debug!("url embed inserted media");
            let embed = Embed {
                id: EmbedId::new(),
                ty: common::v1::types::EmbedType::Media,
                url: Some(url.clone()),
                canonical_url: if url == canonical_url {
                    None
                } else {
                    Some(canonical_url)
                },
                title: None,
                description: None,
                color: None,
                media: Some(media.clone()),
                thumbnail: None,
                author_url: None,
                author_name: None,
                author_avatar: None,
                site_name: None,
                site_avatar: None,
            };

            embed
        } else {
            debug!("got html");

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
            let parsed = HTML::from_string(html.into_owned(), Some(url.to_string()))
                .map_err(Error::UrlEmbed)?;
            debug!("parsed {:?}", parsed);
            let canonical_url = parsed
                .url
                .as_ref()
                .and_then(|u| url.join(u).ok())
                .or_else(|| parsed.meta.get("og:url").and_then(|u| url.join(u).ok()))
                .or_else(|| {
                    parsed
                        .meta
                        .get("twitter:url")
                        .and_then(|u| url.join(u).ok())
                })
                .unwrap_or(fetched.url().to_owned());
            let title = parsed
                .opengraph
                .properties
                .get("title")
                .or(parsed.title.as_ref())
                .or_else(|| parsed.meta.get("twitter:title"))
                .map(ToOwned::to_owned);
            let description = parsed
                .opengraph
                .properties
                .get("description")
                .or(parsed.description.as_ref())
                .or_else(|| parsed.meta.get("twitter:description"))
                .map(ToOwned::to_owned);
            let site_name = parsed
                .opengraph
                .properties
                .get("site_name")
                .map(ToOwned::to_owned);
            let theme_color = parsed
                .opengraph
                .properties
                .get("theme-color")
                .or_else(|| parsed.meta.get("theme-color"))
                .or_else(|| parsed.meta.get("msapplication-TileColor"))
                .and_then(|s| csscolorparser::parse(s).ok());
            // let author = parsed.meta.get("author")
            //     .map(ToOwned::to_owned);
            let m = get_media(&url, &parsed);
            // let m_img = get_img(&url, &parsed);
            let og_type: OpenGraphType = parsed.opengraph.og_type.as_str().into();

            let media_type = match parsed.meta.get("twitter:card").map(|s| s.as_str()) {
                Some("summary_large_image" | "player") => ImageInstructions::Full,
                Some(_) => ImageInstructions::Thumb,
                None => {
                    let robots_instructions: Vec<&str> = parsed
                        .meta
                        .get("robots")
                        .map(|s| s.split(",").map(|s| s.trim()).collect())
                        .unwrap_or_default();
                    // also: nosnippet, max-snippet:100, max-video-preview:100
                    if robots_instructions.contains(&"max-image-preview:none") {
                        ImageInstructions::Hide
                    } else if robots_instructions.contains(&"max-image-preview:standard") {
                        ImageInstructions::Full
                    } else if robots_instructions.contains(&"max-image-preview:large") {
                        ImageInstructions::Thumb
                    } else if og_type.is_media_probably_thumbnail() {
                        ImageInstructions::Thumb
                    } else {
                        ImageInstructions::Full
                    }
                }
            };

            let media = if let Some(m) = m {
                let media = srv
                    .media
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
                    .await?;
                Some(media)
            } else {
                None
            };

            // let media_thumbnail = if let Some(m) = m {
            //     Some(
            //         srv.media
            //             .import_from_url_with_max_size(
            //                 user_id,
            //                 types::MediaCreate {
            //                     alt: m_img.alt,
            //                     source: types::MediaCreateSource::Download {
            //                         filename: None,
            //                         size: None,
            //                         source_url: m_img.url,
            //                     },
            //                 },
            //                 MAX_SIZE_ATTACHMENT,
            //             )
            //             .await?,
            //     )
            // } else {
            //     None
            // };

            let embed = Embed {
                id: EmbedId::new(),
                ty: common::v1::types::EmbedType::Link,
                url: Some(url.clone()),
                canonical_url: if url == canonical_url {
                    None
                } else {
                    Some(canonical_url)
                },
                title,
                description,
                color: theme_color.map(|c| Color::from_hex_string(c.to_css_hex())),
                media: if media_type == ImageInstructions::Full {
                    media.clone()
                } else {
                    None
                },
                thumbnail: if media_type == ImageInstructions::Thumb {
                    media.clone()
                } else {
                    None
                },
                // TODO: parse author information
                author_url: None,
                author_name: None,
                author_avatar: None,
                site_name,
                // TODO: fetch favicon
                site_avatar: None,
            };

            embed
        };
        debug!("done! {:?}", embed);
        Ok(embed)
    }

    async fn attach_embed(
        state: &Arc<ServerStateInner>,
        message_ref: Option<MessageRef>,
        user_id: UserId,
        embed: Embed,
    ) -> Result<()> {
        let Some(mref) = message_ref else {
            return Ok(());
        };
        let data = state.data();
        let message = data
            .message_version_get(mref.thread_id, mref.version_id, user_id)
            .await?;

        let mut message_type = message.message_type;
        let (embeds, attachments) = match &mut message_type {
            MessageType::DefaultMarkdown(m) => {
                if m.embeds
                    .iter()
                    .any(|e| e.url.as_ref() == embed.url.as_ref())
                {
                    info!(
                        "skip embed message: version_id = {} url = {:?}",
                        mref.version_id,
                        embed.url.as_ref().map(|u| u.as_str())
                    );
                    return Ok(());
                }

                if let Some(media) = &embed.media {
                    data.media_link_insert(media.id, *mref.version_id, MediaLinkType::Embed)
                        .await?;
                }
                if let Some(media) = &embed.thumbnail {
                    data.media_link_insert(media.id, *mref.version_id, MediaLinkType::Embed)
                        .await?;
                }
                if let Some(media) = &embed.author_avatar {
                    data.media_link_insert(media.id, *mref.version_id, MediaLinkType::Embed)
                        .await?;
                }
                if let Some(media) = &embed.site_avatar {
                    data.media_link_insert(media.id, *mref.version_id, MediaLinkType::Embed)
                        .await?;
                }

                info!(
                    "add embed message: version_id = {} url = {:?}",
                    mref.version_id,
                    embed.url.as_ref().map(|u| u.as_str())
                );

                m.embeds.push(embed);
                (
                    m.embeds.clone(),
                    m.attachments.iter().map(|a| a.id).collect(),
                )
            }
            _ => return Ok(()),
        };

        data.message_update_in_place(
            mref.thread_id,
            mref.version_id,
            DbMessageCreate {
                channel_id: mref.thread_id,
                attachment_ids: attachments,
                author_id: message.author_id,
                embeds,
                message_type,
                edited_at: message.edited_at.map(|t| t.into()),
                created_at: message.created_at.map(|t| t.into()),
                mentions: message.mentions,
            },
        )
        .await?;

        let mut message = data
            .message_get(mref.thread_id, mref.message_id, user_id)
            .await?;
        if message.version_id == mref.version_id {
            state.presign_message(&mut message).await?;
            state
                .broadcast_channel(
                    mref.thread_id,
                    user_id,
                    MessageSync::MessageUpdate { message },
                )
                .await?;
        } else {
            info!("not sending update because message is not latest");
        }
        Ok(())
    }
}

#[derive(Debug)]
struct ParsedMedia {
    url: Url,
    alt: Option<String>,
}

fn get_media(base: &Url, parsed: &HTML) -> Option<ParsedMedia> {
    for vid in &parsed.opengraph.videos {
        let c: Option<MediaType> = vid
            .properties
            .get("type")
            .and_then(|s| MediaType::parse(s).ok());
        if c.is_none_or(|c| c.ty == "video") {
            return Some(ParsedMedia {
                url: base.join(&vid.url).ok()?,
                alt: vid.properties.get("alt").map(|s| s.to_owned()),
            });
        }
    }

    match get_img(base, parsed) {
        Some(media) => return Some(media),
        None => {}
    }

    for aud in &parsed.opengraph.audios {
        let c: Option<MediaType> = aud
            .properties
            .get("type")
            .and_then(|s| MediaType::parse(s).ok());
        if c.is_none_or(|c| c.ty == "audio") {
            return Some(ParsedMedia {
                url: base.join(&aud.url).ok()?,
                alt: aud.properties.get("alt").map(|s| s.to_owned()),
            });
        }
    }

    None
}

fn get_img(base: &Url, parsed: &HTML) -> Option<ParsedMedia> {
    for img in &parsed.opengraph.images {
        let c: Option<MediaType> = img
            .properties
            .get("type")
            .and_then(|s| MediaType::parse(s).ok());
        if c.is_none_or(|c| c.ty == "image") {
            return Some(ParsedMedia {
                url: base.join(&img.url).ok()?,
                alt: img.properties.get("alt").map(|s| s.to_owned()),
            });
        }
    }

    None
}

fn is_media(m: MediaTypeBuf) -> bool {
    m.ty().as_str() != "text"
}
