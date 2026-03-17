use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, Uri},
    response::{IntoResponse, Response},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use common::v1::types::{InviteCode, InviteTarget};
use lamprey_backend::ServerState;
use minijinja::{context, Environment};
use rand::RngCore;
use rust_embed::RustEmbed;
use serde::Serialize;

use crate::Result;

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

pub async fn frontend_handler(
    uri: Uri,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let mut path = uri.path().trim_start_matches('/').to_string();
    if path.is_empty() {
        path = "index.html".to_string();
    }

    if path != "index.html" {
        if let Some(content) = Asset::get(&path) {
            return Ok(Response::builder()
                .header(header::CONTENT_TYPE, mime_from_ext(&path))
                .body(Body::from(content.data))
                .map_err(|e| Error::FrontendResponseBuilder(e.to_string()))?);
        }
    }

    let nonce = make_nonce();

    let webui_config = WebuiConfig {
        api_url: s.config.api_url.to_string(),
        sync_url: s.config.api_url.join("/api/v1/sync")?.to_string(),
        html_url: s.config.html_url.to_string(),
        cdn_url: s.config.cdn_url.to_string(),
    };

    let env_json = serde_json::to_string(&webui_config)?;

    let env = Environment::new();

    let tpl = Asset::get("index.html")
        .ok_or_else(|| Error::FrontendAssetNotFound("index.html".to_string()))?;
    let template = std::str::from_utf8(tpl.data.as_ref())?;

    let rendered = env
        .render_str(
            template,
            context! {
                nonce => nonce,
                env   => env_json,
            },
        )
        .map_err(|e| Error::FrontendTemplate(e.to_string()))?;

    let rendered = rendered.replace(
        "<!-- VITE_JINJA_PLACEHOLDER:script -->",
        &format!(r#"<script nonce="{nonce}">globalThis.ENV = {env_json};</script>"#),
    );

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "text/html")
        .body(Body::from(rendered))
        .map_err(|e| Error::FrontendResponseBuilder(e.to_string()))?)
}

pub async fn invite_meta_handler(
    Path(code): Path<String>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let invite = d.invite_select(InviteCode(code.clone())).await?;

    let title = match &invite.invite.target {
        InviteTarget::Room { room, .. } => {
            format!("you have been invited to {}", room.name)
        }
        InviteTarget::Gdm { channel } => {
            format!("you have been invited to {}", channel.name)
        }
        InviteTarget::Server => "you have been invited to a server".to_string(),
        InviteTarget::User { user } => {
            format!("{} sent a friend request", user.name)
        }
    };

    let nonce = make_nonce();

    let webui_config = WebuiConfig {
        api_url: s.config.api_url.to_string(),
        sync_url: s.config.api_url.join("/api/v1/sync")?.to_string(),
        html_url: s.config.html_url.to_string(),
        cdn_url: s.config.cdn_url.to_string(),
    };

    let env_json = serde_json::to_string(&webui_config)?;

    let env = Environment::new();

    let tpl = Asset::get("index.html")
        .ok_or_else(|| Error::FrontendAssetNotFound("index.html".to_string()))?;
    let template = std::str::from_utf8(tpl.data.as_ref())?;

    let mut rendered = env
        .render_str(
            template,
            context! {
                nonce => nonce,
                env   => env_json,
            },
        )
        .map_err(|e| Error::FrontendTemplate(e.to_string()))?;

    let og_tags = format!(
        r##"<meta property="og:title" content="{}">
<meta property="og:type" content="website">
<meta property="og:site_name" content="lamprey mountain">
<meta name="twitter:card" content="summary">
<meta name="twitter:title" content="{}">
<meta name="theme-color" content="#b18cf3">"##,
        title, title
    );

    if let Some(viewport_pos) = rendered.find("<meta name=\"viewport\"") {
        if let Some(newline_pos) = rendered[viewport_pos..].find('\n') {
            let insert_pos = viewport_pos + newline_pos + 1;
            rendered.insert_str(insert_pos, &format!("{}\n", og_tags));
        }
    }

    rendered = rendered.replace(
        "<!-- VITE_JINJA_PLACEHOLDER:script -->",
        &format!(r#"<script nonce="{nonce}">globalThis.ENV = {env_json};</script>"#),
    );

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "text/html")
        .body(Body::from(rendered))
        .map_err(|e| Error::FrontendResponseBuilder(e.to_string()))?)
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

// let csp = format!(
//     "default-src 'self'; \
//      script-src 'self' 'nonce-{}'; \
//      style-src 'self' 'unsafe-inline'; \
//      img-src 'self' data:; \
//      connect-src 'self' {}; \
//      font-src 'self'; \
//      object-src 'none'; \
//      base-uri 'none'; \
//      frame-ancestors 'none'",
//     nonce,
//     s.config.api_url,
// );

// Response::builder()
//     .status(StatusCode::OK)
//     .header(header::CONTENT_TYPE, "text/html")
//     .header("Content-Security-Policy", csp)
//     .body(Body::from(rendered))
//     .unwrap()
// }
