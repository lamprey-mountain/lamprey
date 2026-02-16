use std::sync::Arc;

use axum::{
    body::Body,
    http::{header, Uri},
    response::{IntoResponse, Response},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use lamprey_backend::ServerState;
use minijinja::{context, Environment};
use rand::{rngs::OsRng, RngCore};
use rust_embed::RustEmbed;
use serde::Serialize;

use crate::Result;

#[derive(RustEmbed)]
#[folder = "$RUST_EMBED_FRONTEND_PATH"]
struct Asset;

/// minimal data the webui needs needed to start up
#[derive(Serialize)]
struct WebuiConfig {
    api_url: String,
    sync_url: String,
    html_url: String,
    cdn_url: String,
}

// TODO: error variants instead of unwrap
pub async fn frontend_handler(uri: Uri, s: Arc<ServerState>) -> Result<impl IntoResponse> {
    let mut path = uri.path().trim_start_matches('/').to_string();
    if path.is_empty() {
        path = "index.html".to_string();
    }

    if let Some(content) = Asset::get(&path) {
        return Ok(Response::builder()
            .header(header::CONTENT_TYPE, mime_from_ext(&path))
            .body(Body::from(content.data))
            .unwrap());
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

    let tpl = Asset::get("index-jinja.html").unwrap();
    let template = std::str::from_utf8(tpl.data.as_ref()).unwrap();

    let rendered = env
        .render_str(
            template,
            context! {
                nonce => nonce,
                env   => env_json,
            },
        )
        .unwrap();

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "text/html")
        .body(Body::from(rendered))
        .unwrap())
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
