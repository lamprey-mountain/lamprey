use std::convert::Infallible;
use std::{collections::HashMap, str::FromStr, sync::Arc};

use crate::prelude::*;
use crate::routes::util::audit::{AuditTxn, AuditTxnHandle, AuditTxnSlot};
use crate::routes::util::audit_old::AuditLoggerTransaction;
use crate::routes::util::auth::Auth4;
use crate::routes::util::headers::{ContentType, HeadersRequest};
use crate::routes::util::multipart::{MultipartCollector, MultipartFieldName, MultipartFiles};
use crate::ServerState;
use axum::extract::{FromRequest, FromRequestParts};
use bytes::Bytes;
use common::v1::types::AuditLogEntryType;
use common::{
    util::FederationBody,
    v1::{
        routes::ExtractableRoute,
        types::error::{ApiError, ErrorCode, ErrorField, ErrorFieldType},
        RoomId,
    },
    v2::types::media::{Media, MediaReference},
};
use futures::stream;
use multer::Multipart;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::Value;

/// extracts **everything**
///
/// - handles `multipart/form-data` media uploads
/// - produces fancy error messages
// TODO: rename so something more clear?
pub struct UniversalExtractor<T> {
    /// auth state
    pub auth: Auth4,

    /// the main request body
    pub body: T,

    /// resolved media
    media: HashMap<MediaReference, Media>,

    audit_txn_slot: AuditTxnSlot,
}

impl<T> UniversalExtractor<T> {
    pub fn into_inner(self) -> T {
        self.body
    }

    // // TODO: remove
    // pub fn into_parts(self) -> (T, MultipartFiles) {
    //     let files = MultipartFiles { inner: self.files };
    //     (self.body, files)
    // }

    // pub fn get_media(&self, media_ref: &MediaReference) -> &Media {
    //     todo!()
    // }
}

#[derive(Debug, thiserror::Error)]
pub enum ExtractorError {
    /// invalid content type header
    #[error("invalid content-type")]
    InvalidContentType,

    /// missing content type header
    #[error("missing content-type")]
    MissingContentType,

    /// a multipart field as no name
    #[error("a multipart field has no name")]
    MultipartNamelessField,

    /// multipart payload already exists
    #[error("multipart duplicate payload")]
    MultipartDuplicatePayload,

    /// multipart media already exists
    #[error("multipart duplicate media")]
    MultipartDuplicateMedia,

    /// multipart field already exists
    #[error("multipart duplicate field")]
    MultipartDuplicateField,

    /// missing body
    #[error("missing body")]
    MissingBody,
}

impl From<ExtractorError> for Error {
    fn from(value: ExtractorError) -> Self {
        Error::Internal(value.to_string())
    }
}

impl<Req> FromRequest<Arc<ServerState>> for UniversalExtractor<Req>
where
    Req: ExtractableRoute,
    Req::Body: Send,
{
    type Rejection = Error;

    // TODO: import media automatically
    async fn from_request(req: axum::extract::Request, state: &Arc<ServerState>) -> Result<Self> {
        let (mut parts, body) = req.into_parts();
        let bytes = if let Some(body) = parts.extensions.get::<FederationBody>() {
            body.0.clone()
        } else {
            axum::body::to_bytes(body, usize::MAX)
                .await
                // TODO: better error messages
                .map_err(|err| Error::Internal(err.to_string()))?
        };

        let headers = HeadersRequest::from_request_parts(&mut parts, state).await?;
        let auth = Auth4::calculate(&parts, state).await?;
        let audit_txn: &AuditTxnSlot = parts.extensions.get().expect("always exists");
        let audit_txn_slot = Arc::clone(audit_txn);

        match headers.content_type {
            ContentType::Json => {
                let body: Req::Body = parse_json(&bytes)?;
                let req = Req::extract(parts, body).map_err(Error::Response)?;
                Ok(Self {
                    auth,
                    body: req,
                    media: HashMap::new(),
                    audit_txn_slot,
                })
            }
            ContentType::Msgpack => {
                let body: Req::Body = parse_msgpack(&bytes)?;
                let req = Req::extract(parts, body).map_err(Error::Response)?;
                Ok(Self {
                    auth,
                    body: req,
                    media: HashMap::new(),
                    audit_txn_slot,
                })
            }
            ContentType::Multipart => {
                let boundary =
                    multer::parse_boundary(todo!("get actual content type header from parts"))?;
                let stream = stream::once(async move { Ok::<Bytes, std::io::Error>(bytes) });
                let multipart = multer::Multipart::new(stream, boundary);
                let collector = MultipartCollector::collect(multipart).await?;
                let (body, media) = collector.parse()?;
                let req = Req::extract(parts, body).map_err(Error::Response)?;
                Ok(UniversalExtractor {
                    auth,
                    body: req,
                    media,
                    audit_txn_slot,
                })
            }
            ContentType::Invalid => Err(Error::BadStatic("invalid content-type header")),
            ContentType::Missing => {
                if bytes.is_empty() {
                    // try to deserialize from "null" in case Req::Body == ()
                    let body = serde_json::from_str("null").map_err(|_| {
                        Error::BadStatic("route requires a body but none was provided")
                    })?;
                    let req = Req::extract(parts, body).map_err(Error::Response)?;
                    Ok(UniversalExtractor {
                        auth,
                        body: req,
                        media: HashMap::new(),
                        audit_txn_slot,
                    })
                } else {
                    Err(Error::BadStatic("missing content-type header"))
                }
            }
        }
    }
}

impl<Req> UniversalExtractor<Req> {
    // TODO: don't require calculating AuditLogEntryType up front
    #[must_use = "must call commit() to save a successful audit log entry"]
    pub async fn begin_audit_log(
        &self,
        context_id: RoomId,
        ty: AuditLogEntryType,
    ) -> Result<AuditTxnHandle> {
        let txn = self.audit_txn_slot.lock().await;
        txn.begin(context_id);
        // let txn = AuditLoggerTransaction {
        //     context_id,
        //     auth: self.clone(),
        //     reason: self.reason.clone(),
        //     started_at: Time::now_utc(),
        //     application_id: self.session.app_id,
        //     ty: Some(ty),
        //     status: None,
        // };

        Ok(AuditTxnHandle {
            slot: Arc::clone(&self.audit_txn_slot),
        })
    }
}

pub(crate) fn parse_json<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
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

pub(crate) fn parse_msgpack<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    todo!()
}
