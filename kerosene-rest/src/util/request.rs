use crate::{
    prelude::*,
    util::headers::{ContentType, HeadersRequest},
};

use axum::extract::FromRequest;
use common::{
    util::routes::Endpoint,
    v1::{
        routes::ExtractableRequest,
        types::error::{ErrorField, ErrorFieldType},
    },
};
use kerosene_core::{error::ErrorCode, types::auth::Identity};
use lamprey_backend_services::services::Services;
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
    media: (),

    reason: Option<String>,
    // headers: (),
    // audit_txn_slot: AuditTxnSlot,
}

impl<E> FromRequest<Globals> for Req<E>
where
    E: Endpoint + Send,
    E::Request: Send,
{
    type Rejection = ServerError;

    async fn from_request(req: axum::extract::Request, globals: &Globals) -> ServerResult<Self> {
        let (parts, body) = req.into_parts();
        let headers = HeadersRequest::from_parts(&parts)?;
        let identity = super::auth::calculate(&headers, globals).await?;

        // FIXME: federation
        let body = axum::body::to_bytes(body, usize::MAX)
            .await
            // TODO: better errors
            .map_err(|err| Error::Internal(Box::new(err)))?;

        match headers.content_type {
            ContentType::Json => {
                // let body: Req::Body = parse_json(&bytes)?;
                // let req = Req::extract(parts, body).map_err(Error::Response)?;
                // Ok(Self {
                //     auth,
                //     body: req,
                //     media: Default::default(),
                //     reason: headers.reason,
                //     audit_txn_slot,
                // })
                todo!()
            }
            ContentType::Form => todo!(),
            ContentType::Msgpack => todo!(),
            ContentType::Multipart => todo!(),
            ContentType::Invalid => todo!(),
            ContentType::Missing => todo!(),
        };

        Ok(Self {
            inner: todo!(),
            globals: globals.clone(),
            identity,
            media: todo!(),
            reason: headers.reason,
        })
    }
}

impl<E: Endpoint> Req<E> {
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

    // pub fn get_media(&self, media_ref: &MediaReference) -> &Media {
    //     todo!()
    // }

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

fn parse_json<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let jd = &mut serde_json::Deserializer::from_slice(bytes);
    let data: T = match serde_path_to_error::deserialize(jd) {
        Ok(data) => data,
        Err(err) => {
            // TODO: multiple error fields
            return Err(ApiError {
                message: err.to_string(),
                fields: vec![ErrorField {
                    key: err.path().iter().map(|s| s.to_string()).collect(),
                    message: err.to_string(),
                    ty: ErrorFieldType::Other,
                }],
                ..ApiError::from_code(ErrorCode::InvalidData)
            }
            .into());
        }
    };

    Ok(data)
}

fn parse_form<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let s = std::str::from_utf8(bytes).map_err(|err| ApiError {
        message: err.to_string(),
        fields: vec![],
        ..ApiError::from_code(ErrorCode::InvalidData)
    })?;
    let data: T = serde_urlencoded::from_str(s).map_err(|err| ApiError {
        message: err.to_string(),
        fields: vec![],
        ..ApiError::from_code(ErrorCode::InvalidData)
    })?;
    Ok(data)
}

fn parse_msgpack<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let data: T = rmp_serde::from_slice(bytes).map_err(|err| ApiError {
        message: err.to_string(),
        fields: vec![],
        ..ApiError::from_code(ErrorCode::InvalidData)
    })?;
    Ok(data)
}
