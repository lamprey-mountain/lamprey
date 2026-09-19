use std::{collections::HashMap, error::Error, sync::Arc};

use crate::{
    prelude::*,
    util::{
        error::{ExtractorError, ExtractorRejection},
        headers::{ContentType, HeadersRequest},
        multipart::MultipartCollector,
        parse::{parse_form, parse_json, parse_msgpack},
    },
};

use axum::{
    extract::{FromRequest, rejection::LengthLimitError},
    response::IntoResponse,
};
use common::{
    util::routes::Endpoint,
    v1::{
        routes::ExtractableRequest,
        types::error::{ErrorField, ErrorFieldType},
    },
    v2::types::media::{Media, MediaReference},
};
use futures::stream;
use http::StatusCode;
use kerosene_core::{error::ErrorCode, types::auth::Identity};
use lamprey_backend_services::services::{
    Services,
    media::{Import, MediaItem},
};

/// the current state for a request
///
/// can be used as an axum extractor
pub struct Req<E: Endpoint> {
    inner: E::Request,

    /// a handle to global state
    globals: Globals,

    /// the identity of who is making this request
    identity: Identity,

    /// resolved media
    media: HashMap<MediaReference, MediaItem>,

    reason: Option<String>,
    // headers: (),
    // audit_txn_slot: AuditTxnSlot,
}

impl<E> FromRequest<Globals> for Req<E>
where
    E: Endpoint + Send,
    E::Request: ExtractableRequest + Send,
{
    type Rejection = ExtractorRejection;

    async fn from_request(
        req: axum::extract::Request,
        globals: &Globals,
    ) -> CoreResult<Self, ExtractorRejection> {
        let (parts, body) = req.into_parts();
        let headers = HeadersRequest::from_parts(&parts)?;
        let identity = super::auth::calculate(&headers, globals).await?;

        // FIXME: federation
        let body = axum::body::to_bytes(body, usize::MAX)
            .await
            .map_err(|err| {
                if err.source().is_some_and(|s| s.is::<LengthLimitError>()) {
                    ExtractorRejection::ExtractorError(ExtractorError::BodyTooLarge)
                } else {
                    ExtractorRejection::ServerError(ServerError::Internal(Box::new(err)))
                }
            })?;

        let (inner, media) = match headers.content_type {
            ContentType::Json => {
                let deserialized: <E::Request as ExtractableRequest>::Body = parse_json(&body)?;
                (E::Request::extract(parts, deserialized)?, HashMap::new())
            }
            ContentType::Form => {
                let deserialized: <E::Request as ExtractableRequest>::Body = parse_form(&body)?;
                (E::Request::extract(parts, deserialized)?, HashMap::new())
            }
            ContentType::Msgpack => {
                let deserialized: <E::Request as ExtractableRequest>::Body = parse_msgpack(&body)?;
                (E::Request::extract(parts, deserialized)?, HashMap::new())
            }
            ContentType::Multipart => {
                let ct = parts
                    .headers
                    .get("content-type")
                    .expect("must have existed earlier")
                    .to_str()
                    .map_err(|_| {
                        ExtractorRejection::ExtractorError(ExtractorError::InvalidContentType)
                    })?;
                let boundary = multer::parse_boundary(ct).map_err(|e| {
                    // TODO: custom error for bad multipart boundary
                    ExtractorRejection::ServerError(ServerError::Internal(Box::new(e)))
                })?;
                let stream = stream::once(async move { Ok::<Bytes, std::io::Error>(body) });
                let multipart = multer::Multipart::new(stream, boundary);
                let collector = MultipartCollector::collect(multipart).await?;
                let (body, files) = collector.parse()?;
                let inner = E::Request::extract(parts, body)?;

                // import media
                let srv = globals.services();
                let mut media = HashMap::new();
                if !files.is_empty() {
                    let user = identity.ensure_user().map_err(|e| {
                        ExtractorRejection::ServerError(ServerError::Api(Box::new(e)))
                    })?;

                    // PERF: import in parallel
                    for (num, file) in files {
                        let import = Import::new(user.id);
                        let item = srv
                            .media
                            .import_from_multipart(import, file)
                            .await
                            .map_err(|e| {
                                ExtractorRejection::ServerError(ServerError::Internal(Box::new(e)))
                            })?;
                        media.insert(MediaReference::Attachment { media_index: num }, item);
                    }
                }
                (inner, media)
            }
            ContentType::Invalid => {
                return Err(ExtractorError::InvalidContentType.into());
            }
            ContentType::Missing => {
                if body.is_empty() {
                    // try to deserialize from "null" for endpoints with no body (Body == ())
                    let deserialized =
                        serde_json::from_str("null").map_err(|_| ExtractorError::MissingBody)?;
                    (E::Request::extract(parts, deserialized)?, HashMap::new())
                } else {
                    return Err(ExtractorError::MissingContentType.into());
                }
            }
        };

        Ok(Self {
            inner,
            globals: globals.clone(),
            identity,
            media,
            reason: headers.reason,
        })
    }
}

impl<E: Endpoint> Req<E> {
    /// access server global state
    #[inline]
    pub fn globals(&self) -> Globals {
        self.globals.clone()
    }

    #[inline]
    pub fn services(&self) -> Arc<Services> {
        self.globals.services()
    }

    #[inline]
    pub fn identity(&self) -> &Identity {
        &self.identity
    }

    #[inline]
    pub fn inner(&self) -> &E::Request {
        &self.inner
    }

    #[inline]
    pub fn into_inner(self) -> E::Request {
        self.inner
    }

    pub fn get_media(&self, media_ref: &MediaReference) -> Option<&MediaItem> {
        self.media.get(media_ref)
    }

    // /// begin an audit log transaction
    // #[must_use = "must call commit() to save a successful audit log entry"]
    // pub async fn begin_audit_log(
    //     &self,
    //     room_id: RoomId,
    //     ty: AuditLogEntryType,
    // ) -> Result<AuditTxnHandle> {
    //     todo!()
    // }
}
