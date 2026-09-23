use std::{any::TypeId, collections::HashMap, error::Error, sync::Arc};

use crate::{
    prelude::*,
    util::{
        audit_log::{ActorInfo, AuditLoggerHandle, AuditLoggerSlot},
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
    util::{body::Body, routes::Endpoint},
    v1::{
        routes::ExtractableRequest,
        types::error::{ErrorField, ErrorFieldType},
    },
    v2::types::media::{Media, MediaReference},
};
use futures::stream;
use http::{Method, StatusCode};
use kerosene_core::{error::ErrorCode, types::auth::Identity};
use lamprey_backend_services::services::{
    Services,
    media::{Import, MediaItem},
};
use serde::de::DeserializeOwned;

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

    /// request headers
    headers: Box<HeadersRequest>,

    /// request method
    method: Method,

    /// an audit logger slot
    audit_logger: AuditLoggerSlot,
}

impl<E> FromRequest<Globals> for Req<E>
where
    E: Endpoint + Send,
    E::Request: ExtractableRequest + Send,
    <E::Request as ExtractableRequest>::Body: DeserializeOwned,
{
    type Rejection = ExtractorRejection;

    async fn from_request(
        req: axum::extract::Request,
        globals: &Globals,
    ) -> CoreResult<Self, ExtractorRejection> {
        let (parts, body) = req.into_parts();
        let headers = HeadersRequest::from_parts(&parts)?;
        let identity = super::auth::calculate(&headers, globals).await?;
        let method = parts.method.clone();

        let audit_logger: &AuditLoggerSlot = parts
            .extensions
            .get()
            .expect("audit logger slot should always exist");
        let audit_logger = Arc::clone(audit_logger);

        // FIXME: support federation
        let body = axum::body::to_bytes(body, usize::MAX)
            .await
            .map_err(|err| {
                if err.source().is_some_and(|s| s.is::<LengthLimitError>()) {
                    ExtractorRejection::ExtractorError(ExtractorError::BodyTooLarge)
                } else {
                    ExtractorRejection::ServerError(ServerError::Internal(Box::new(err)))
                }
            })?;

        // FIXME: handle raw body request
        // if TypeId::of::<<E::Request as ExtractableRequest>::Body>() == TypeId::of::<Body>() {
        //     return Ok(Self {
        //         inner: E::Request::extract(parts, body.into())?,
        //         globals: globals.clone(),
        //         identity,
        //         media: HashMap::new(),
        //         headers: Box::new(headers),
        //         audit_logger,
        //     });
        // }

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
            headers: Box::new(headers),
            method,
            audit_logger,
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

    /// access request headers
    #[inline]
    pub fn headers(&self) -> &HeadersRequest {
        &self.headers
    }

    /// access request method
    #[inline]
    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn get_media(&self, media_ref: &MediaReference) -> Option<&MediaItem> {
        self.media.get(media_ref)
    }

    /// obtain a handle to the current audit logger
    pub fn audit_log(&self) -> AuditLoggerHandle {
        let identity = self.identity();
        let session = identity.session().unwrap(); // FIXME: Handle missing session
        let user_id = identity.user_id().unwrap();

        let actor = Arc::new(ActorInfo {
            user_id,
            session_id: session.id,
            application_id: session.app_id,
            user_agent: session.imprint.user_agent.clone(),
            ip_addr: session.imprint.ip_addr.clone(),
        });

        let slot = self.audit_logger.clone();
        AuditLoggerHandle::new(actor, slot)
    }
}
