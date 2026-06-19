use std::{str::FromStr, sync::Arc};

use axum::{
    body::to_bytes,
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use axum_extra::TypedHeader;
use common::v1::types::{
    error::{ApiError, ErrorCode},
    ids::RedexId,
    redex::{EvalInput, EvalStatus},
};
use headers::Host;
use lamprey_script::engine::ExecutionEvent;

use crate::{Error, Result, ServerState};

pub async fn script_http(
    TypedHeader(host): TypedHeader<Host>,
    State(state): State<Arc<ServerState>>,
    req: Request,
    next: Next,
) -> Result<Response> {
    let Some(suffix) = &state.config.scripts.suffix else {
        return Ok(next.run(req).await);
    };

    let Some(prefix) = host.hostname().strip_suffix(suffix) else {
        return Ok(next.run(req).await);
    };

    let Ok(script_id) = RedexId::from_str(prefix) else {
        return Ok(next.run(req).await);
    };

    let mut data = state.data();
    let script = data
        .script_get(script_id)
        .await?
        .ok_or(Error::BadStatic("script not found"))?;

    let (parts, body) = req.into_parts();
    let body_bytes = to_bytes(body, 1024 * 1024 * 16)
        .await
        .map_err(|e| Error::Internal(format!("Failed to read body: {}", e)))?;
    let req_for_script = Request::from_parts(parts, body_bytes);

    let srv = state.services();
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
