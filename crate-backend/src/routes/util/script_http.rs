use std::str::FromStr;

use axum::{
    body::to_bytes,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{Html, IntoResponse, Response},
};
use axum_extra::TypedHeader;
use common::v1::types::{
    error::{ApiError, ErrorCode},
    ids::RedexId,
    redex::{EvalInput, EvalStatus},
};
use headers::Host;
use lamprey_script::engine::ExecutionEvent;

use crate::prelude::*;

pub async fn script_http(
    TypedHeader(host): TypedHeader<Host>,
    State(globals): State<Globals>,
    req: Request,
    next: Next,
) -> Result<Response> {
    let Some(suffix) = globals.config().scripts.public_suffix() else {
        return Ok(next.run(req).await);
    };

    let host = host.hostname().trim_end_matches('.').to_ascii_lowercase();
    let suffix = suffix.trim_matches('.').to_ascii_lowercase();
    if suffix.is_empty() {
        return Ok(next.run(req).await);
    }

    if host == suffix {
        if req.uri().path() == "/" {
            return redex_root(globals.config().html_url.as_str());
        }

        return Ok(next.run(req).await);
    }

    let Some(prefix) = host.strip_suffix(&suffix) else {
        return Ok(next.run(req).await);
    };
    let Some(prefix) = prefix.strip_suffix('.') else {
        return Ok(next.run(req).await);
    };

    if prefix.is_empty()
        || prefix.contains('.')
        || !prefix
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return redex_not_found(globals.config().html_url.as_str());
    }

    let mut data = globals.begin_read().await?;
    let script = if let Ok(script_id) = RedexId::from_str(prefix) {
        data.script_get(script_id).await?
    } else {
        data.script_get_by_shortname(prefix).await?
    };
    let Some(script) = script else {
        return redex_not_found(globals.config().html_url.as_str());
    };

    let script_id = script.id;

    let (parts, body) = req.into_parts();
    let body_bytes = to_bytes(body, 1024 * 1024 * 16)
        .await
        .map_err(|e| Error::Internal(format!("Failed to read body: {}", e)))?;
    let req_for_script = Request::from_parts(parts, body_bytes);

    let srv = globals.services();
    let redex_version_id = script.latest_version.version_id;
    let mut handle = srv
        .scripts
        .spawn(
            script.channel_id,
            script_id,
            redex_version_id,
            EvalInput::Http {
                request: req_for_script,
            },
        )
        .await?;

    while let Ok(event) = handle.poll().await {
        match &*event {
            ExecutionEvent::HttpResponse(res) => {
                let (parts, bytes) = res.to_owned().into_parts();
                let res = http::Response::from_parts(parts, axum::body::Body::from(bytes));
                return Ok(res);
            }
            ExecutionEvent::Status(EvalStatus::Crashed) => {
                return Err(Error::ApiError(ApiError::with_message(
                    ErrorCode::ScriptError,
                    "script crashed while generating a response".to_string(),
                )));
            }
            ExecutionEvent::Status(EvalStatus::Exited) => break,
            _ => {}
        }
    }

    Err(Error::ApiError(ApiError::with_message(
        ErrorCode::ScriptError,
        "script failed to respond with a response".to_string(),
    )))
}

fn render_page(name: &str, template: &str, html_url: &str, status: StatusCode) -> Result<Response> {
    let rendered = minijinja::Environment::new()
        .render_named_str(
            name,
            template,
            minijinja::context! {
                html_url => html_url,
            },
        )
        .map_err(|e| Error::Internal(format!("failed to render {name}: {e}")))?;

    Ok((status, Html(rendered)).into_response())
}

fn redex_root(html_url: &str) -> Result<Response> {
    render_page(
        "redex_root.html",
        include_str!("../../html/redex_root.html"),
        html_url,
        StatusCode::OK,
    )
}

fn redex_not_found(html_url: &str) -> Result<Response> {
    render_page(
        "redex_not_found.html",
        include_str!("../../html/redex_not_found.html"),
        html_url,
        StatusCode::NOT_FOUND,
    )
}
