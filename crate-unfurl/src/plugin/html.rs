use std::{ops::ControlFlow, str::FromStr};

use html5ever::{
    local_name,
    tokenizer::{TagKind, Token},
};
use lamprey_common::v1::types::{EmbedType, misc::Color};
use url::Url;

use crate::{
    Plugin, PluginHtml, Unfurler,
    plugin::{
        Head, HtmlSink, UnfurlResult,
        html::util::{RobotsImagePreview, TwitterCard},
    },
    unfurler::{EmbedGeneration, UnfurlerBuilder},
    util::{EmbedGenerationTemplate, EmbedMediaPending},
};

mod util;

pub struct HtmlStreamPlugin {
    pub max_bytes: usize,
}

impl Plugin for HtmlStreamPlugin {
    fn register(self, builder: UnfurlerBuilder) -> UnfurlerBuilder {
        builder.add_plugin_html(self)
    }
}

impl PluginHtml for HtmlStreamPlugin {
    fn create_sink(&self, _unfurler: &Unfurler, head: &Head<'_>) -> Box<dyn HtmlSink> {
        Box::new(HtmlStreamSink {
            data: ExtractedData::default(),
            url: head.url.clone(),
        })
    }
}

/// Image display mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImageMode {
    /// Don't show image
    Hide,

    /// Show as full-width main media
    Full,

    /// Show as thumbnail
    Thumb,
}

/// Determine image display mode based on Twitter card and robots directives
// TODO: move to util
fn determine_image_mode(data: &ExtractedData) -> ImageMode {
    // Check robots max-image-preview first (takes precedence)
    if let Some(robots) = data.robots_max_image_preview {
        return match robots {
            RobotsImagePreview::None => ImageMode::Hide,
            RobotsImagePreview::Standard => ImageMode::Thumb,
            RobotsImagePreview::Large => ImageMode::Full,
        };
    }

    // Check Twitter card
    match data.twitter_card {
        Some(TwitterCard::SummaryLargeImage | TwitterCard::Player) => ImageMode::Full,
        Some(TwitterCard::Summary | TwitterCard::App) => ImageMode::Thumb,
        // Default: use OG type heuristics
        None => {
            let og_type = data.og_type.as_deref().unwrap_or("website");
            if is_og_type_probably_thumbnail(og_type) {
                ImageMode::Thumb
            } else {
                ImageMode::Full
            }
        }
    }
}

/// Check if an OG type typically uses thumbnail images
// TODO: move to util
fn is_og_type_probably_thumbnail(og_type: &str) -> bool {
    matches!(
        og_type,
        "music.song"
            | "music.album"
            | "music.playlist"
            | "music.radio_station"
            | "article"
            | "book"
            | "profile"
            | "website"
    )
}

/// Parse robots max-image-preview directive from robots meta content
// TODO: move to util
fn parse_robots_image_preview(content: &str) -> Option<RobotsImagePreview> {
    // Robots meta can contain multiple comma-separated directives
    // e.g., "noindex, nofollow, max-image-preview:large"
    for directive in content.split(',') {
        let directive = directive.trim().to_lowercase();
        if directive.starts_with("max-image-preview:") {
            let value = directive.strip_prefix("max-image-preview:")?.trim();
            // TODO: move to FromStr impl
            return Some(match value {
                "none" => RobotsImagePreview::None,
                "standard" => RobotsImagePreview::Standard,
                "large" => RobotsImagePreview::Large,
                _ => continue,
            });
        }
    }
    None
}

#[derive(Debug, Default)]
struct ExtractedData {
    in_title: bool,
    current_title: String,

    in_ld_json: bool,
    current_ld_json: String,
    ld_json: Vec<String>,

    title: Option<String>,
    og_title: Option<String>,
    twitter_title: Option<String>,

    description: Option<String>,
    og_description: Option<String>,
    twitter_description: Option<String>,

    og_site_name: Option<String>,

    favicon_url: Option<String>,
    apple_touch_icon_url: Option<String>,

    author_name: Option<String>,
    author_url: Option<String>,
    author_avatar: Option<String>,

    canonical_url: Option<String>,
    og_url: Option<String>,
    theme_color: Option<String>,

    og_type: Option<String>,
    images: MediaList,
    audios: MediaList,
    videos: MediaList,

    feeds: Vec<String>,
    rel_me: Vec<String>,

    twitter_card: Option<TwitterCard>,
    robots_max_image_preview: Option<RobotsImagePreview>,
}

impl ExtractedData {
    pub fn has_meaningful_embed(&self) -> bool {
        let has_enough_html_meta = self.title.is_some() && self.description.is_some();
        self.og_title.is_some()
            || self.og_description.is_some()
            || self.twitter_title.is_some()
            || self.twitter_description.is_some()
            || self.has_media()
            || has_enough_html_meta
    }

    pub fn has_media(&self) -> bool {
        !self.images.entries.is_empty()
            || !self.videos.entries.is_empty()
            || !self.audios.entries.is_empty()
    }
}

#[derive(Debug, Default)]
struct MediaList {
    entries: Vec<MediaEntry>,
}

#[derive(Debug, Default, Clone)]
struct MediaEntry {
    url: Option<String>,
    secure_url: Option<String>,
    ty: Option<String>,
    width: Option<String>,
    height: Option<String>,
    alt: Option<String>,
}

impl MediaEntry {
    pub fn preferred_url(&self) -> Option<&str> {
        self.secure_url.as_deref().or(self.url.as_deref())
    }
}

impl MediaList {
    /// handle a meta tag
    ///
    pub fn handle_meta(&mut self, suffix: &str, content: String) {
        match suffix {
            "" | ":url" => {
                let lm = self.last_media();
                if lm.url.is_some() {
                    self.push_media().url = Some(content);
                } else {
                    lm.url = Some(content);
                }
            }
            ":secure_url" => {
                let lm = self.last_media();
                if lm.secure_url.is_some() {
                    self.push_media().secure_url = Some(content);
                } else {
                    lm.secure_url = Some(content);
                }
            }
            ":type" => {
                self.last_media().ty = Some(content);
            }
            ":alt" => {
                self.last_media().alt = Some(content);
            }
            ":width" => {
                self.last_media().width = Some(content);
            }
            ":height" => {
                self.last_media().height = Some(content);
            }
            _ => {}
        }
    }

    /// get the last MediaEntry, creating it if it doesnt exist
    fn last_media(&mut self) -> &mut MediaEntry {
        if self.entries.is_empty() {
            self.push_media();
        }
        self.entries.last_mut().unwrap()
    }

    /// create a new MediaEntry and return it
    fn push_media(&mut self) -> &mut MediaEntry {
        self.entries.push_mut(MediaEntry::default())
    }
}

struct HtmlStreamSink {
    data: ExtractedData,
    url: Url,
}

impl HtmlSink for HtmlStreamSink {
    fn push(&mut self, token: &Token, _line_number: u64) -> ControlFlow<UnfurlResult> {
        match token {
            Token::TagToken(tag) => {
                if tag.kind == TagKind::StartTag {
                    match tag.name {
                        local_name!("title") => {
                            self.data.in_title = true;
                            self.data.current_title.clear();
                        }
                        local_name!("meta") => {
                            let mut name = None;
                            let mut property = None;
                            let mut content = None;

                            for attr in tag.attrs.iter() {
                                match attr.name.local {
                                    local_name!("name") => name = Some(attr.value.to_string()),
                                    local_name!("property") => {
                                        property = Some(attr.value.to_string())
                                    }
                                    local_name!("content") => {
                                        content = Some(attr.value.to_string())
                                    }
                                    _ => {}
                                }
                            }

                            if let Some(content) = content {
                                let key = property.or(name).unwrap_or_default().to_lowercase();

                                if let Some(s) = key.strip_prefix("og:video") {
                                    self.data.videos.handle_meta(s, content);
                                } else if let Some(s) = key.strip_prefix("og:image") {
                                    self.data.images.handle_meta(s, content);
                                } else if let Some(s) = key.strip_prefix("og:audio") {
                                    self.data.audios.handle_meta(s, content);
                                } else {
                                    match key.as_str() {
                                        "og:title" => self.data.og_title = Some(content),
                                        "twitter:title" => self.data.twitter_title = Some(content),
                                        "twitter:card" => {
                                            self.data.twitter_card = content.parse().ok()
                                        }
                                        "description" => self.data.description = Some(content),
                                        "og:description" => {
                                            self.data.og_description = Some(content)
                                        }
                                        "twitter:description" => {
                                            self.data.twitter_description = Some(content)
                                        }
                                        "og:site_name" => self.data.og_site_name = Some(content),
                                        "theme-color" => self.data.theme_color = Some(content),
                                        "og:url" => self.data.og_url = Some(content),
                                        "og:type" => self.data.og_type = Some(content),
                                        "robots" => {
                                            self.data.robots_max_image_preview =
                                                parse_robots_image_preview(&content);
                                        }

                                        "author" => self.data.author_name = Some(content),
                                        "article:author" => {
                                            if self.data.author_name.is_none() {
                                                self.data.author_name = Some(content);
                                            }
                                        }

                                        // NOTE: do i need something like ogp logic for these?
                                        "profile:image" | "twitter:creator:image" => {
                                            self.data.author_avatar = Some(content)
                                        }

                                        // TODO: make sure image logic is correct?
                                        "twitter:image" => {
                                            self.data.images.last_media().url = Some(content);
                                        }

                                        _ => {}
                                    }
                                }
                            }
                        }
                        local_name!("link") => {
                            let mut rel = None;
                            let mut href = None;
                            let mut ty = None;

                            for attr in tag.attrs.iter() {
                                match attr.name.local {
                                    local_name!("rel") => rel = Some(attr.value.to_string()),
                                    local_name!("href") => href = Some(attr.value.to_string()),
                                    local_name!("type") => ty = Some(attr.value.to_string()),
                                    _ => {}
                                }
                            }

                            if let (Some(rel), Some(href)) = (rel, href) {
                                let rels: Vec<&str> = rel.split_whitespace().collect();

                                for r in rels {
                                    match r.to_lowercase().as_str() {
                                        "icon" => {
                                            if self.data.favicon_url.is_none() {
                                                self.data.favicon_url = Some(href.clone());
                                            }
                                        }
                                        "apple-touch-icon" => {
                                            self.data.apple_touch_icon_url = Some(href.clone());
                                        }
                                        "canonical" => self.data.canonical_url = Some(href.clone()),
                                        "author" => self.data.author_url = Some(href.clone()),
                                        "me" => self.data.rel_me.push(href.clone()),
                                        "alternate" => {
                                            if let Some(t) = &ty {
                                                let lt = t.to_lowercase();
                                                if lt == "application/rss+xml"
                                                    || lt == "application/atom+xml"
                                                    || lt == "application/json"
                                                {
                                                    self.data.feeds.push(href.clone());
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                        local_name!("script") => {
                            for attr in tag.attrs.iter() {
                                if attr.name.local == local_name!("type")
                                    && attr.value.to_lowercase() == "application/ld+json"
                                {
                                    self.data.in_ld_json = true;
                                    self.data.current_ld_json.clear();
                                }
                            }
                        }
                        _ => {}
                    }
                } else if tag.kind == TagKind::EndTag {
                    if tag.name == local_name!("title") {
                        self.data.in_title = false;
                        if self.data.title.is_none() {
                            let title = self.data.current_title.trim().to_string();
                            self.data.title = Some(title);
                        }
                    } else if tag.name == local_name!("script") {
                        if self.data.in_ld_json {
                            self.data.in_ld_json = false;
                            let s = self.data.current_ld_json.clone();
                            self.data.ld_json.push(s);
                        }
                    }
                }
            }
            Token::CharacterTokens(s) => {
                if self.data.in_title {
                    self.data.current_title.push_str(&s);
                } else if self.data.in_ld_json {
                    self.data.current_ld_json.push_str(&s);
                }
            }
            _ => {}
        }

        ControlFlow::Continue(())
    }

    fn finish(self: Box<Self>) -> UnfurlResult {
        let data = self.data;

        if !data.has_meaningful_embed() {
            return UnfurlResult::skip();
        }

        let image_mode = determine_image_mode(&data);

        let mut tmpl = EmbedGenerationTemplate {
            ty: EmbedType::Link,
            url: Some(self.url.clone()),
            canonical_url: data.canonical_url.and_then(|u| self.url.join(&u).ok()),
            title: data.og_title.or(data.twitter_title).or(data.title),
            description: data.og_description.or(data.description),
            site_name: data.og_site_name,
            color: data
                .theme_color
                .as_ref()
                .and_then(|c| Color::from_str(c).ok()),
            media: None,
            thumbnail: None,
            author_name: None,
            author_url: None,
            author_avatar: None,
            site_avatar: None,
            // TODO: handle favicon as site_avatar
            // i need some way to avoid constantly refetching the same favicon though...
            // site_avatar: data.favicon_url.and_then(|u| url.join(&u).ok()).map(|u| {
            //     EmbedMediaPending::new(u)
            //         .mime_guess("image/x-icon".parse().unwrap())
            //         .into()
            // }),
        };

        // Handle nested/recursive media
        let og_type = data.og_type.as_deref().unwrap_or("website");
        let is_media = matches!(
            og_type,
            "video"
                | "video.movie"
                | "video.episode"
                | "video.tv_show"
                | "video.other"
                | "music.song"
                | "music.album"
                | "music.playlist"
                | "music.radio_station"
        );

        // TODO: parse mime types from url? likely unnecessary if the media importer system autodetects mime anyways
        if is_media && !data.videos.entries.is_empty() {
            // TODO: don't only check the first video, iterate until a usable video is found
            let video_meta = data.videos.entries.first();
            if video_meta.and_then(|m| m.ty.as_deref()) == Some("text/html") {
                // html videos (iframes) aren't allowed
                tmpl.ty = EmbedType::Link;
            } else {
                tmpl.ty = EmbedType::Media;
                if let Some(v_url) = data.videos.entries[0]
                    .preferred_url()
                    .and_then(|u| self.url.join(u).ok())
                {
                    tmpl.media = Some(
                        EmbedMediaPending::new(v_url)
                            .mime_guess("video/mp4".parse().unwrap())
                            .into(),
                    );
                }
            }

            // TODO: don't only check the first image, iterate until a usable image is found
            if let Some(img) = data.images.entries.first() {
                if let Some(img_url) = img.preferred_url() {
                    if let Ok(i_url) = self.url.join(img_url) {
                        tmpl.thumbnail = Some(
                            EmbedMediaPending::new(i_url)
                                .mime_guess("image/jpeg".parse().unwrap())
                                .into(),
                        );
                    }
                }
            }
        } else if let Some(img) = data.images.entries.first() {
            // TODO: don't only check the first image, iterate until a usable image is found
            if let Some(img_url) = img.preferred_url() {
                if let Ok(i_url) = self.url.join(img_url) {
                    // NOTE: there might be cases where i want to include full media *and* a thumbnail?
                    match image_mode {
                        ImageMode::Hide => {}
                        ImageMode::Full => {
                            tmpl.media = Some(
                                EmbedMediaPending::new(i_url)
                                    .mime_guess("image/jpeg".parse().unwrap())
                                    .into(),
                            );
                        }
                        ImageMode::Thumb => {
                            tmpl.thumbnail = Some(
                                EmbedMediaPending::new(i_url)
                                    .mime_guess("image/jpeg".parse().unwrap())
                                    .into(),
                            );
                        }
                    }
                }
            }
        }

        // TODO: handle rel=me and RSS feeds if needed later...

        UnfurlResult {
            embeds: vec![EmbedGeneration { embed: tmpl }],
            errors: vec![],
            stop: false,
        }
    }
}
