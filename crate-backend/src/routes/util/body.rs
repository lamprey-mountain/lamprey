use std::{collections::HashMap, str::FromStr, sync::Arc};

use crate::{Error, Result, ServerState};
use axum::extract::FromRequest;
use bytes::Bytes;
use common::{
    util::FederationBody,
    v1::{
        routes::ExtractableRoute,
        types::error::{ApiError, ErrorCode, ErrorField, ErrorFieldType},
    },
};
use futures::stream;
use serde::de::DeserializeOwned;
use serde_json::Value;

/// extract request body
///
/// - handles `multipart/form-data` media uploads
/// - produces fancy error messages
// TODO: handle non-json to make it truly universal?
pub struct UniversalExtractor<T> {
    /// main request body
    req: T,

    /// additional files from multipart
    files: HashMap<u64, MultipartFile>,
}

pub enum MultipartFieldName {
    /// embedded json `payload_json`
    PayloadJson,

    /// media to upload, `media[n]`
    Media(u64),

    /// field `anything_else`
    Field(String),
}

#[derive(Debug)]
pub struct MultipartCollector<T> {
    payload_json: Option<T>,
    fields: Vec<(String, Value)>,
    media: HashMap<u64, MultipartFile>,
}

#[derive(Debug)]
pub struct MultipartFiles {
    pub inner: HashMap<u64, MultipartFile>,
}

/// a file uploaded via multipart/form-data
#[derive(Debug)]
pub struct MultipartFile {
    pub filename: Option<String>,
    pub content_type: Option<String>,
    pub data: Bytes,
}

impl<T> Default for MultipartCollector<T> {
    fn default() -> Self {
        Self {
            payload_json: None,
            fields: vec![],
            media: HashMap::new(),
        }
    }
}

impl<T> UniversalExtractor<T> {
    pub fn into_inner(self) -> T {
        self.req
    }

    pub fn into_parts(self) -> (T, MultipartFiles) {
        let files = MultipartFiles { inner: self.files };
        (self.req, files)
    }
}

impl FromStr for MultipartFieldName {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        if s == "payload_json" {
            return Ok(MultipartFieldName::PayloadJson);
        }

        if let Some(rest) = s.strip_prefix("media[") {
            if let Some(n_str) = rest.strip_suffix(']') {
                if let Ok(n) = n_str.parse::<u64>() {
                    return Ok(MultipartFieldName::Media(n));
                }
            }
        }

        Ok(MultipartFieldName::Field(s.to_owned()))
    }
}

impl<Req> FromRequest<Arc<ServerState>> for UniversalExtractor<Req>
where
    Req: ExtractableRoute,
    Req::Body: Send,
{
    type Rejection = Error;

    async fn from_request(req: axum::extract::Request, _state: &Arc<ServerState>) -> Result<Self> {
        let (parts, body) = req.into_parts();
        let bytes = if let Some(body) = parts.extensions.get::<FederationBody>() {
            body.0.clone()
        } else {
            axum::body::to_bytes(body, usize::MAX)
                .await
                // TODO: better error messages
                .map_err(|err| Error::Internal(err.to_string()))?
        };

        if let Some(ct) = parts.headers.get("content-type") {
            let ct = mediatype::MediaType::parse(ct.to_str()?)?;

            match ct.essence() {
                ct if ct == mediatype::media_type!(APPLICATION / JSON) => {
                    let body: Req::Body = parse_json(&bytes)?;
                    let req = match Req::extract(parts, body) {
                        Ok(req) => req,
                        Err(res) => return Err(Error::Response(res)),
                    };
                    Ok(Self {
                        req,
                        files: HashMap::new(),
                    })
                }
                ct if ct == mediatype::media_type!(MULTIPART / FORM_DATA) => {
                    let boundary = multer::parse_boundary(ct.to_string())?;
                    let stream = stream::once(async move { Ok::<Bytes, std::io::Error>(bytes) });
                    let mut multipart = multer::Multipart::new(stream, boundary);
                    let mut collector = MultipartCollector::default();

                    while let Some(field) = multipart.next_field().await? {
                        let name: MultipartFieldName = field
                            .name()
                            .ok_or(Error::BadStatic("multipart field is missing name"))?
                            .parse()?;

                        let content_type = field.content_type().map(|s| s.to_owned());
                        let file_name = field.file_name().map(|s| s.to_owned());
                        let data = field.bytes().await?;

                        match name {
                            MultipartFieldName::PayloadJson => {
                                let json: Req::Body = parse_json(&data)?;
                                collector.payload_json = Some(json);
                            }
                            MultipartFieldName::Media(n) => {
                                let file = MultipartFile {
                                    filename: file_name,
                                    content_type: content_type.map(|c| c.to_string()),
                                    data,
                                };
                                if collector.media.insert(n, file).is_some() {
                                    return Err(Error::BadStatic("media already exists"));
                                }
                            }
                            MultipartFieldName::Field(name) => {
                                let json: Value = parse_json(&data)?;
                                collector.fields.push((name, json));
                            }
                        }
                    }

                    // TODO: merge `fields` into `payload_json`
                    let Some(body) = collector.payload_json else {
                        return Err(Error::BadStatic("missing payload_json"));
                    };

                    let req = match Req::extract(parts, body) {
                        Ok(req) => req,
                        Err(res) => return Err(Error::Response(res)),
                    };

                    Ok(UniversalExtractor {
                        req,
                        files: collector.media,
                    })
                }
                _ => Err(Error::BadStatic("unsupported content-type ")),
            }
        } else {
            if bytes.is_empty() {
                // try to deserialize from "null" in case Req::Body == ()
                let body = serde_json::from_str("null")
                    .map_err(|_| Error::BadStatic("route requires a body but none was provided"))?;

                let req = match Req::extract(parts, body) {
                    Ok(req) => req,
                    Err(res) => return Err(Error::Response(res)),
                };

                Ok(UniversalExtractor {
                    req,
                    files: HashMap::new(),
                })
            } else {
                Err(Error::BadStatic("missing content-type header"))
            }
        }
    }
}

fn parse_json<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let jd = &mut serde_json::Deserializer::from_slice(bytes);
    let data: T = match serde_path_to_error::deserialize(jd) {
        Ok(data) => data,
        Err(err) => {
            // TODO: multiple error fields
            return Err(Error::ApiError(ApiError {
                message: err.to_string(),
                fields: vec![ErrorField {
                    key: err.path().iter().map(|s| s.to_string()).collect(),
                    message: err.to_string(),
                    ty: ErrorFieldType::Other,
                }],
                ..ApiError::from_code(ErrorCode::InvalidData)
            }));
        }
    };

    Ok(data)
}
