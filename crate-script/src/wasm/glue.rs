use bytes::Bytes;

use crate::wasm::wit::{HttpRequest, HttpResponse};

impl From<http::Request<Bytes>> for HttpRequest {
    fn from(value: http::Request<Bytes>) -> Self {
        let (parts, body) = value.into_parts();
        HttpRequest {
            method: parts.method.to_string(),
            url: parts.uri.to_string(),
            headers: parts
                .headers
                .iter()
                .map(|(k, v)| {
                    (
                        k.as_str().to_string(),
                        String::from_utf8_lossy(v.as_bytes()).to_string(),
                    )
                })
                .collect(),
            body: Some(body.to_vec()),
        }
    }
}

impl From<HttpResponse> for http::Response<Bytes> {
    fn from(value: HttpResponse) -> Self {
        let mut builder = http::Response::builder().status(value.status);
        for (k, v) in value.headers {
            builder = builder.header(k, v);
        }
        builder
            .body(Bytes::from_iter(value.body.unwrap_or_default()))
            .unwrap()
    }
}
