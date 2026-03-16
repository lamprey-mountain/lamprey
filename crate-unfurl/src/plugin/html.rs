use std::{cell::RefCell, rc::Rc};

use async_trait::async_trait;
use html5ever::{
    local_name,
    tendril::StrTendril,
    tokenizer::{
        BufferQueue, TagKind, Token, TokenSink, TokenSinkResult, Tokenizer, TokenizerOpts,
    },
};
use lamprey_common::v1::types::{misc::Color, EmbedType};
use reqwest::Response;
use url::Url;

use crate::{
    error::UnfurlError,
    plugin::UnfurlPlugin,
    unfurler::EmbedGeneration,
    util::{EmbedGenerationTemplate, EmbedMediaPending},
};

pub struct HtmlStreamPlugin {
    pub max_bytes: usize,
}

#[async_trait]
impl UnfurlPlugin for HtmlStreamPlugin {
    fn name(&self) -> &'static str {
        "HtmlStreamPlugin"
    }

    fn accepts_response(&self, res: &Response) -> bool {
        res.headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.contains("text/html"))
            .unwrap_or(false) // Default to false, some sites might be sneaky though
    }

    async fn process_response(
        &self,
        url: &Url,
        mut res: Response,
    ) -> Result<Vec<EmbedGeneration>, UnfurlError> {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<bytes::Bytes>(32);

        // the html parser is !Send due to Rc, so spawn a blocking task
        let parse_task = tokio::task::spawn_blocking(move || {
            let shared_data = Rc::new(RefCell::new(ExtractedData::default()));
            let sink = MetaSink {
                data: shared_data.clone(),
            };
            let tokenizer = Tokenizer::new(sink, TokenizerOpts::default());
            let mut queue = BufferQueue::default();

            // In your spawn_blocking task:
            let mut tail = Vec::new();

            while let Some(chunk) = rx.blocking_recv() {
                let s = decode_chunk(&mut tail, &chunk);
                queue.push_back(StrTendril::from_slice(&s));
                let _ = tokenizer.feed(&mut queue);
            }

            tokenizer.end();

            Rc::try_unwrap(shared_data).unwrap().into_inner()
        });

        let mut bytes_read = 0;
        while let Some(chunk) = res.chunk().await? {
            bytes_read += chunk.len();

            if tx.send(chunk).await.is_err() {
                break;
            }

            if bytes_read > self.max_bytes {
                break;
            }
        }

        drop(tx);

        let data = parse_task.await?;

        let mut tmpl = EmbedGenerationTemplate {
            ty: EmbedType::Link,
            url: Some(url.clone()),
            canonical_url: data.canonical_url.and_then(|u| url.join(&u).ok()),
            title: data.og_title.or(data.twitter_title).or(data.title),
            description: data.og_description.or(data.description),
            site_name: data.og_site_name,
            color: data
                .theme_color
                .and_then(|c| Color::try_from_hex_string(c).ok()),
            media: None,
            thumbnail: None,
            author_name: None,
            author_url: None,
            author_avatar: None,
            site_avatar: None,
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
        if is_media && !data.videos.is_empty() {
            tmpl.ty = EmbedType::Media;
            if let Ok(v_url) = url.join(&data.videos[0]) {
                tmpl.media = Some(
                    EmbedMediaPending::new(v_url)
                        .mime_guess("video/mp4".parse().unwrap())
                        .into(),
                );

                if let Some(img) = data.images.first() {
                    if let Ok(i_url) = url.join(img) {
                        tmpl.thumbnail = Some(
                            EmbedMediaPending::new(i_url)
                                .mime_guess("image/jpeg".parse().unwrap())
                                .into(),
                        );
                    }
                }
            }
        } else if let Some(img) = data.images.first() {
            if let Ok(i_url) = url.join(img) {
                // typical website, image becomes a thumbnail
                tmpl.thumbnail = Some(
                    EmbedMediaPending::new(i_url)
                        .mime_guess("image/jpeg".parse().unwrap())
                        .into(),
                );
            }
        }

        // Handle rel=me and RSS feeds if needed later...

        Ok(vec![EmbedGeneration { embed: tmpl }])
    }
}

/// Merges `tail` with `chunk`, returning the valid UTF-8 string and
/// storing any incomplete trailing bytes back into `tail`.
fn decode_chunk<'a>(tail: &mut Vec<u8>, chunk: &[u8]) -> String {
    let bytes = if tail.is_empty() {
        chunk.to_vec()
    } else {
        let mut b = std::mem::take(tail);
        b.extend_from_slice(chunk);
        b
    };

    match std::str::from_utf8(&bytes) {
        Ok(s) => s.to_string(),
        Err(e) => {
            let valid = &bytes[..e.valid_up_to()];
            tail.extend_from_slice(&bytes[e.valid_up_to()..]);
            std::str::from_utf8(valid).unwrap().to_string()
        }
    }
}

#[derive(Default, Debug)]
struct ExtractedData {
    in_title: bool,
    current_title: String,

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
    images: Vec<String>,
    videos: Vec<String>,

    feeds: Vec<String>,
    rel_me: Vec<String>,
}

struct MetaSink {
    data: Rc<RefCell<ExtractedData>>,
}

impl TokenSink for MetaSink {
    type Handle = ();

    // Changed `&mut self` to `&self` to match your trait requirements
    fn process_token(&self, token: Token, _line_number: u64) -> TokenSinkResult<()> {
        match token {
            Token::TagToken(tag) => {
                if tag.kind == TagKind::StartTag {
                    match tag.name {
                        local_name!("title") => {
                            let mut data = self.data.borrow_mut();
                            data.in_title = true;
                            data.current_title.clear();
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
                                let mut data = self.data.borrow_mut();

                                match key.as_str() {
                                    "og:title" => data.og_title = Some(content),
                                    "twitter:title" => data.twitter_title = Some(content),
                                    "description" => data.description = Some(content),
                                    "og:description" => data.og_description = Some(content),
                                    "twitter:description" => {
                                        data.twitter_description = Some(content)
                                    }
                                    "og:site_name" => data.og_site_name = Some(content),
                                    "theme-color" => data.theme_color = Some(content),
                                    "og:url" => data.og_url = Some(content),
                                    "og:type" => data.og_type = Some(content),

                                    "author" => data.author_name = Some(content),
                                    "article:author" => {
                                        if data.author_name.is_none() {
                                            data.author_name = Some(content);
                                        }
                                    }
                                    "profile:image" | "twitter:creator:image" => {
                                        data.author_avatar = Some(content)
                                    }

                                    "og:image" | "twitter:image" => data.images.push(content),
                                    "og:video" | "og:video:url" | "og:video:secure_url" => {
                                        data.videos.push(content)
                                    }
                                    _ => {}
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
                                let mut data = self.data.borrow_mut();
                                let rels: Vec<&str> = rel.split_whitespace().collect();

                                for r in rels {
                                    match r.to_lowercase().as_str() {
                                        "icon" | "shortcut icon" => {
                                            if data.favicon_url.is_none() {
                                                data.favicon_url = Some(href.clone());
                                            }
                                        }
                                        "apple-touch-icon" => {
                                            data.apple_touch_icon_url = Some(href.clone());
                                        }
                                        "canonical" => data.canonical_url = Some(href.clone()),
                                        "author" => data.author_url = Some(href.clone()),
                                        "me" => data.rel_me.push(href.clone()),
                                        "alternate" => {
                                            if let Some(t) = &ty {
                                                let lt = t.to_lowercase();
                                                if lt == "application/rss+xml"
                                                    || lt == "application/atom+xml"
                                                    || lt == "application/json"
                                                {
                                                    data.feeds.push(href.clone());
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                } else if tag.kind == TagKind::EndTag {
                    if tag.name == local_name!("title") {
                        let mut data = self.data.borrow_mut();
                        data.in_title = false;
                        if data.title.is_none() {
                            // Extract title out to avoid borrow issues
                            let title = data.current_title.trim().to_string();
                            data.title = Some(title);
                        }
                    }
                }
            }
            Token::CharacterTokens(s) => {
                let mut data = self.data.borrow_mut();
                if data.in_title {
                    data.current_title.push_str(&s);
                }
            }
            _ => {}
        }
        TokenSinkResult::Continue
    }
}
