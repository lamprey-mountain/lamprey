use std::sync::LazyLock;

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
    routing::get,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use common::{
    v1::types::{
        ApplicationId, Channel, ChannelId, InviteCode, InviteTarget, MediaId,
        MessageAttachmentType, MessageId, MessageType, Room, RoomId, UserId,
    },
    v2::types::media::MediaMetadata,
};
use minijinja::{Environment, context, value::Serde};
use rand::Rng;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};

use crate::{
    Routes,
    prelude::{Error, LegacyErrorExt, Result},
    util::Globals,
};

#[derive(RustEmbed)]
#[folder = "$RUST_EMBED_FRONTEND_PATH"]
struct Asset;

#[derive(Serialize)]
struct WebuiConfig {
    api_url: String,
    sync_url: String,
    html_url: String,
    cdn_url: String,
}

#[derive(Default, Serialize)]
struct PageMeta {
    title: Option<String>,
    description: Option<String>,
    image: Option<String>,
    image_alt: Option<String>,
    image_width: Option<u64>,
    image_height: Option<u64>,
    image_type: Option<String>,
    url: Option<String>,
    og_type: Option<&'static str>,
    profile_username: Option<String>,
    robots: Option<&'static str>,
    #[serde(skip)]
    status: Option<StatusCode>,
}

static META_ENV: LazyLock<Environment<'static>> = LazyLock::new(|| {
    let mut env = Environment::new();
    env.add_template("meta.html", include_str!("frontend/meta.html"))
        .expect("meta.html template must be valid");
    env
});

#[derive(Deserialize)]
struct AuthorizeQuery {
    client_id: Option<ApplicationId>,
}

pub fn register(r: &mut Routes) {
    r.route("/invite/{code}", get(invite_meta_handler));
    r.route("/media/{media_id}", get(media_meta_handler));
    r.route("/user/{user_id}", get(user_meta_handler));
    r.route("/room/{room_id}", get(room_meta_handler));
    r.route("/channel/{channel_id}", get(channel_meta_handler));
    r.route("/thread/{channel_id}", get(channel_meta_handler));
    r.route(
        "/channel/{channel_id}/message/{message_id}",
        get(channel_message_meta_handler),
    );
    r.route(
        "/thread/{channel_id}/message/{message_id}",
        get(channel_message_meta_handler),
    );
    r.route("/authorize", get(authorize_meta_handler));
    r.route("/verify-email", get(noindex_handler));
    r.route("/search", get(noindex_handler));
    r.route("/inbox", get(noindex_handler));
    r.route("/friends", get(noindex_handler));
    r.fallback(frontend_handler);
}

async fn frontend_handler(uri: Uri, State(globals): State<Globals>) -> Result<impl IntoResponse> {
    let config = globals.config();
    let mut path = uri.path().trim_start_matches('/').to_string();
    if path.is_empty() {
        path = "index.html".to_string();
    }

    if path != "index.html" {
        let mut content_data = None;

        if let Some(fs_path) = &config.ui.path {
            let full_path = fs_path.join(&path);
            if let Ok(content) = tokio::fs::read(&full_path).await {
                content_data = Some(content);
            }
        }

        if content_data.is_none() {
            if let Some(content) = Asset::get(&path) {
                content_data = Some(content.data.into_owned());
            }
        }

        if let Some(data) = content_data {
            return Ok(Response::builder()
                .header(header::CONTENT_TYPE, mime_from_ext(&path))
                .body(Body::from(data))
                .map_err(Error::internal)?);
        }
    }

    render_index(&globals, &PageMeta::default()).await
}

async fn noindex_handler(State(globals): State<Globals>) -> Result<impl IntoResponse> {
    render_index(
        &globals,
        &PageMeta {
            robots: Some("noindex, nofollow"),
            ..Default::default()
        },
    )
    .await
}

async fn invite_meta_handler(
    Path(code): Path<InviteCode>,
    State(globals): State<Globals>,
) -> Result<impl IntoResponse> {
    let mut txn = globals.begin_read().await.cast_internal()?;

    let Ok(invite) = txn.invite_get(code.clone()).await else {
        return not_found(&globals).await;
    };
    if invite.is_dead() {
        return not_found(&globals).await;
    }

    let (title, description, image) = match &invite.invite.target {
        InviteTarget::Room { room, .. } => (
            format!("you have been invited to {}", room.name),
            invite
                .invite
                .description
                .clone()
                .or_else(|| room.description.clone()),
            room.icon.or(room.banner),
        ),
        InviteTarget::Gdm { channel } => (
            format!("you have been invited to #{}", channel_name(channel)),
            invite.invite.description.clone(),
            channel.icon,
        ),
        InviteTarget::Server => (
            "you have been invited to a server".to_string(),
            invite.invite.description.clone(),
            None,
        ),
        InviteTarget::User { user } => (
            format!("@{} sent a friend request", user.name),
            invite.invite.description.clone(),
            user.avatar,
        ),
    };

    let config = globals.config();
    let meta = PageMeta {
        title: Some(title),
        description,
        image: image.map(|id| thumb_url(config, id)),
        url: Some(absolute(&config.html_url, &format!("/invite/{}", code.0))),
        robots: Some("noindex, noarchive"),
        ..Default::default()
    };

    render_index(&globals, &meta).await
}

async fn media_meta_handler(
    Path(media_id): Path<MediaId>,
    State(globals): State<Globals>,
) -> Result<impl IntoResponse> {
    let mut txn = globals.begin_read().await.cast_internal()?;
    let Ok(media) = txn.media_select(media_id).await else {
        return not_found(&globals).await;
    };
    if media.deleted_at.is_some() || media.quarantine.is_some() {
        return not_found(&globals).await;
    }

    let config = globals.config();
    let title = media.alt.clone().unwrap_or_else(|| media.filename.clone());
    let description = media.alt.clone().or_else(|| match &media.metadata {
        MediaMetadata::Image { width, height } => Some(format!("image {width}x{height}")),
        MediaMetadata::Video {
            width,
            height,
            duration,
        } => Some(format!("video {width}x{height} {duration}s")),
        MediaMetadata::Audio { duration } => Some(format!("audio {duration}s")),
        MediaMetadata::File => Some("file".to_string()),
        MediaMetadata::Text => Some("text".to_string()),
        MediaMetadata::Errored { .. } => None,
    });

    let (image, image_width, image_height, image_type) = if media.has_thumbnail {
        (Some(thumb_url(config, media_id)), None, None, None)
    } else if media.content_type.to_string().starts_with("image/") {
        let (width, height) = match &media.metadata {
            MediaMetadata::Image { width, height } => (Some(*width), Some(*height)),
            _ => (None, None),
        };
        (
            Some(absolute(&config.cdn_url, &format!("/media/{media_id}"))),
            width,
            height,
            Some(media.content_type.to_string()),
        )
    } else {
        (None, None, None, None)
    };

    let og_type = match &media.metadata {
        MediaMetadata::Video { .. } => "video.other",
        MediaMetadata::Audio { .. } => "music.song",
        _ => "website",
    };

    let meta = PageMeta {
        title: Some(title),
        description,
        image,
        image_alt: media.alt.clone(),
        image_width,
        image_height,
        image_type,
        url: Some(absolute(&config.html_url, &format!("/media/{media_id}"))),
        og_type: Some(og_type),
        ..Default::default()
    };

    render_index(&globals, &meta).await
}

async fn user_meta_handler(
    Path(user_id): Path<UserId>,
    State(globals): State<Globals>,
) -> Result<impl IntoResponse> {
    let mut txn = globals.begin_read().await.cast_internal()?;
    let Ok(user) = txn.user_get(user_id).await else {
        return not_found(&globals).await;
    };
    if user.deleted_at.is_some() || user.suspended.is_some() || user.remote.is_some() {
        return not_found(&globals).await;
    }

    let config = globals.config();
    let meta = PageMeta {
        title: Some(format!("@{}", user.name)),
        description: user.description.clone(),
        image: user.avatar.map(|id| thumb_url(config, id)),
        image_alt: Some(format!("Avatar for @{}", user.name)),
        url: Some(absolute(&config.html_url, &format!("/user/{user_id}"))),
        og_type: Some("profile"),
        profile_username: Some(user.name),
        ..Default::default()
    };

    render_index(&globals, &meta).await
}

async fn room_meta_handler(
    Path(room_id): Path<RoomId>,
    State(globals): State<Globals>,
) -> Result<impl IntoResponse> {
    let mut txn = globals.begin_read().await.cast_internal()?;
    let Ok(room) = txn.room_get(room_id).await else {
        return not_found(&globals).await;
    };
    if !room.public || room.deleted_at.is_some() {
        return not_found(&globals).await;
    }

    let config = globals.config();
    let meta = PageMeta {
        title: Some(room.name.clone()),
        description: room.description.clone().or_else(|| {
            Some(format!(
                "{} members · {} online",
                room.member_count, room.online_count
            ))
        }),
        image: room.icon.or(room.banner).map(|id| thumb_url(config, id)),
        image_alt: Some(room.name.clone()),
        url: Some(absolute(&config.html_url, &format!("/room/{room_id}"))),
        og_type: Some("website"),
        ..Default::default()
    };

    render_index(&globals, &meta).await
}

async fn channel_meta_handler(
    Path(channel_id): Path<ChannelId>,
    State(globals): State<Globals>,
) -> Result<impl IntoResponse> {
    let Some((channel, room)) = public_channel(&globals, channel_id).await? else {
        return not_found(&globals).await;
    };

    let name = channel_name(&channel);
    let config = globals.config();
    let meta = PageMeta {
        title: Some(format!("#{} - {}", name, room.name)),
        description: channel
            .description
            .clone()
            .or_else(|| channel.auto_description.clone()),
        image: channel.icon.or(room.icon).map(|id| thumb_url(config, id)),
        image_alt: Some(format!("Icon for #{}", name)),
        url: Some(absolute(
            &config.html_url,
            &format!("/channel/{channel_id}"),
        )),
        og_type: Some("website"),
        ..Default::default()
    };

    render_index(&globals, &meta).await
}

// TODO: fancier message rendering, dynamically render message content to an image
// unsure what the performance implications of this would be
async fn channel_message_meta_handler(
    Path((channel_id, message_id)): Path<(ChannelId, MessageId)>,
    State(globals): State<Globals>,
) -> Result<impl IntoResponse> {
    let Some((channel, room)) = public_channel(&globals, channel_id).await? else {
        return not_found(&globals).await;
    };

    let mut txn = globals.begin_read().await.cast_internal()?;
    let Ok(message) = txn.message_get(channel_id, message_id).await else {
        return not_found(&globals).await;
    };
    if message.deleted_at.is_some() || message.removed_at.is_some() || message.ephemeral {
        return not_found(&globals).await;
    }

    let author = txn
        .user_get(message.author_id)
        .await
        .map(|user| user.name)
        .unwrap_or_else(|_| "someone".to_string());

    let (content, attachments) = match &message.latest_version.message_type {
        MessageType::DefaultMarkdown(message) | MessageType::ThreadInitial(message) => {
            (message.content.clone(), message.attachments.as_slice())
        }
        _ => (None, &[][..]),
    };

    // FIXME: use og:video for video attachments
    let config = globals.config();
    let (image, image_alt, image_type) = attachments
        .iter()
        .find_map(|attachment| match &attachment.ty {
            MessageAttachmentType::Media { media } => {
                let content_type = media.content_type.to_string();
                if !content_type.starts_with("image/") && !content_type.starts_with("video/") {
                    return None;
                }
                let (image, image_type) = if media.has_thumbnail {
                    (thumb_url(config, media.id), None)
                } else if content_type.starts_with("image/") {
                    (
                        absolute(&config.cdn_url, &format!("/media/{}", media.id)),
                        Some(content_type.clone()),
                    )
                } else {
                    return None;
                };
                Some((
                    image,
                    media.alt.clone().or_else(|| Some(media.filename.clone())),
                    image_type,
                ))
            }
            _ => None,
        })
        .map(|(image, image_alt, image_type)| (Some(image), image_alt, image_type))
        .unwrap_or_default();

    let name = channel_name(&channel);
    let meta = PageMeta {
        title: Some(format!("Message by @{} in #{}", author, name)),
        description: content.map(|content| truncate(&content, 200)),
        image,
        image_alt,
        image_type,
        url: Some(absolute(
            &config.html_url,
            &format!("/channel/{channel_id}/message/{message_id}"),
        )),
        og_type: Some("article"),
        ..Default::default()
    };

    render_index(&globals, &meta).await
}

async fn authorize_meta_handler(
    State(globals): State<Globals>,
    Query(query): Query<AuthorizeQuery>,
) -> Result<impl IntoResponse> {
    let Some(application_id) = query.client_id else {
        return not_found(&globals).await;
    };

    let mut txn = globals.begin_read().await.cast_internal()?;
    let Ok(application) = txn.application_get(application_id).await else {
        return not_found(&globals).await;
    };
    if !application.public {
        return not_found(&globals).await;
    }

    let config = globals.config();
    let image = txn
        .user_get(UserId::from(application.id.into_inner()))
        .await
        .ok()
        .and_then(|user| user.avatar)
        .map(|id| thumb_url(config, id));

    let meta = PageMeta {
        title: Some(format!("Authorize {}", application.name)),
        description: application
            .description
            .clone()
            .or_else(|| Some("An application wants to access your account".to_string())),
        image,
        image_alt: Some(application.name.clone()),
        url: Some(absolute(&config.html_url, "/authorize")),
        og_type: Some("website"),
        robots: Some("noindex, nofollow"),
        ..Default::default()
    };

    render_index(&globals, &meta).await
}

async fn public_channel(
    globals: &Globals,
    channel_id: ChannelId,
) -> Result<Option<(Channel, Room)>> {
    let mut txn = globals.begin_read().await.cast_internal()?;
    let Ok(channel) = txn.channel_get(channel_id).await else {
        return Ok(None);
    };
    let Some(room_id) = channel.room_id else {
        return Ok(None);
    };
    let Ok(room) = txn.room_get(room_id).await else {
        return Ok(None);
    };
    if !room.public || room.deleted_at.is_some() || channel.deleted_at.is_some() {
        return Ok(None);
    }
    Ok(Some((channel, room)))
}

async fn not_found(globals: &Globals) -> Result<Response> {
    render_index(
        globals,
        &PageMeta {
            robots: Some("noindex, nofollow"),
            status: Some(StatusCode::NOT_FOUND),
            ..Default::default()
        },
    )
    .await
}

async fn render_index(globals: &Globals, meta: &PageMeta) -> Result<Response> {
    let config = globals.config();
    let nonce = make_nonce();

    let webui_config = WebuiConfig {
        api_url: config.api_url.to_string(),
        sync_url: config
            .api_url
            .join("/api/v1/sync")
            .map_err(Error::internal)?
            .to_string(),
        html_url: config.html_url.to_string(),
        cdn_url: config.cdn_url.to_string(),
    };

    let env_json = serde_json::to_string(&webui_config).map_err(Error::internal)?;
    let env = Environment::new();

    let tpl_data = if let Some(fs_path) = &config.ui.path {
        tokio::fs::read(fs_path.join("index.html"))
            .await
            .map_err(Error::internal)?
    } else {
        Asset::get("index.html")
            .ok_or_else(|| Error::internal_message("frontend index.html is missing"))?
            .data
            .into_owned()
    };
    let template = std::str::from_utf8(&tpl_data).map_err(Error::internal)?;

    let mut rendered = env
        .render_str(
            template,
            context! {
                nonce => &nonce,
                env   => &env_json,
            },
        )
        .map_err(Error::internal)?;

    if let Some(title) = &meta.title {
        let title = META_ENV
            .render_named_str(
                "title.html",
                "<title>{{ title }}</title>",
                context! { title => title },
            )
            .map_err(Error::internal)?;
        rendered = rendered.replace("<title>Lamprey Mountain</title>", &title);
    }

    let meta_tags = META_ENV
        .get_template("meta.html")
        .and_then(|template| template.render(Serde(meta)))
        .map_err(Error::internal)?;
    // PERF: use template for meta and script (.replace allocs a new string each time)
    rendered = rendered.replace("<!-- VITE_JINJA_PLACEHOLDER:meta -->", &meta_tags);
    rendered = rendered.replace(
        "<!-- VITE_JINJA_PLACEHOLDER:script -->",
        &format!(r#"<script nonce="{nonce}">globalThis.ENV = {env_json};</script>"#),
    );

    Ok(Response::builder()
        .status(meta.status.unwrap_or(StatusCode::OK))
        .header(header::CONTENT_TYPE, "text/html")
        .body(Body::from(rendered))
        .map_err(Error::internal)?)
}

fn channel_name(channel: &Channel) -> String {
    if channel.name.as_deref().is_none_or(str::is_empty) {
        channel
            .auto_name
            .clone()
            .unwrap_or_else(|| "unknown-channel".to_string())
    } else {
        channel.name.clone().unwrap_or_default()
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if value.chars().count() <= max_chars {
        value
    } else {
        let mut truncated: String = value.chars().take(max_chars).collect();
        truncated.push('…');
        truncated
    }
}

fn absolute(base: &url::Url, path: &str) -> String {
    format!("{}{}", base.as_str().trim_end_matches('/'), path)
}

fn thumb_url(config: &kerosene_core::config::Config, media_id: MediaId) -> String {
    absolute(&config.cdn_url, &format!("/thumb/{media_id}?size=64"))
}

fn mime_from_ext(path: &str) -> &'static str {
    match path.split('.').last() {
        Some("html") => "text/html",
        Some("css") => "text/css",
        Some("js") => "application/javascript",
        Some("wasm") => "application/wasm",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") => "image/jpeg",
        Some("ico") => "image/x-icon",
        _ => "application/octet-stream",
    }
}

fn make_nonce() -> String {
    let mut bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut bytes);
    STANDARD.encode(bytes)
}
