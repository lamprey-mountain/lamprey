use axum::{
    body::Body,
    extract::Request,
    http::{HeaderValue, header},
    middleware::Next,
    response::Response,
};
use common::util::{body::Body as CommonBody, routes::EncodableResponse};

use crate::util::headers::Accept;

/// encode an endpoint response in the format preferred by the request
pub fn encode<R>(response: R, accept: Accept) -> http::Response<CommonBody>
where
    R: EncodableResponse,
    R::Body: serde::Serialize,
{
    match accept {
        Accept::Msgpack => response.encode_with("application/msgpack", |body| {
            rmp_serde::to_vec_named(body).unwrap()
        }),
        Accept::Json => response.encode(),
    }
}

/// middleware that converts json error responses to msgpack when the client
/// prefers it.
///
/// successful responses are encoded directly from their typed json body by
/// [`encode`], so they keep the correct serde representation (for example,
/// binary values are sent as msgpack bytes instead of base64).
pub async fn msgpack_errors(req: Request, next: Next) -> Response {
    let accept = Accept::from_headers(req.headers());
    let res = next.run(req).await;

    if accept != Accept::Msgpack || res.status().is_success() {
        return res;
    }

    let is_json = res
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("application/json"));

    if !is_json {
        return res;
    }

    let (mut parts, body) = res.into_parts();

    let Ok(bytes) = axum::body::to_bytes(body, usize::MAX).await else {
        return Response::from_parts(parts, Body::empty());
    };

    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return Response::from_parts(parts, Body::from(bytes));
    };

    let Ok(msgpack) = rmp_serde::to_vec_named(&value) else {
        return Response::from_parts(parts, Body::from(bytes));
    };

    parts.headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/msgpack"),
    );

    Response::from_parts(parts, Body::from(msgpack))
}
