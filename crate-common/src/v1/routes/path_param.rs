use crate::v1::types::{
    document::DocumentRevisionId, reaction::ReactionKeyParam, room_template::RoomTemplateCode, Id,
    InviteCode,
};

/// Trait for types that can be parsed from a path parameter string
pub trait PathParam: Sized {
    fn from_path_param(s: &str) -> Result<Self, PathParamError>;
}

/// Error type for path parameter extraction
#[derive(Debug)]
pub struct PathParamError(pub String);

impl From<PathParamError> for http::Response<bytes::Bytes> {
    fn from(err: PathParamError) -> Self {
        http::Response::builder()
            .status(http::StatusCode::BAD_REQUEST)
            .body(bytes::Bytes::from(err.0))
            .unwrap()
    }
}

impl PathParam for String {
    fn from_path_param(s: &str) -> Result<Self, PathParamError> {
        Ok(s.to_string())
    }
}

impl PathParam for i64 {
    fn from_path_param(s: &str) -> Result<Self, PathParamError> {
        s.parse()
            .map_err(|_| PathParamError(format!("invalid i64: {}", s)))
    }
}

impl PathParam for i32 {
    fn from_path_param(s: &str) -> Result<Self, PathParamError> {
        s.parse()
            .map_err(|_| PathParamError(format!("invalid i32: {}", s)))
    }
}

impl PathParam for u64 {
    fn from_path_param(s: &str) -> Result<Self, PathParamError> {
        s.parse()
            .map_err(|_| PathParamError(format!("invalid u64: {}", s)))
    }
}

impl PathParam for u32 {
    fn from_path_param(s: &str) -> Result<Self, PathParamError> {
        s.parse()
            .map_err(|_| PathParamError(format!("invalid u32: {}", s)))
    }
}

impl<M: crate::v1::types::ids::Marker> PathParam for Id<M> {
    fn from_path_param(s: &str) -> Result<Self, PathParamError> {
        s.parse()
            .map_err(|_| PathParamError(format!("invalid id: {}", s)))
    }
}

impl PathParam for RoomTemplateCode {
    fn from_path_param(s: &str) -> Result<Self, PathParamError> {
        Ok(RoomTemplateCode(s.to_string()))
    }
}

impl PathParam for InviteCode {
    fn from_path_param(s: &str) -> Result<Self, PathParamError> {
        Ok(InviteCode(s.to_string()))
    }
}

impl PathParam for uuid::Uuid {
    fn from_path_param(s: &str) -> Result<Self, PathParamError> {
        s.parse()
            .map_err(|_| PathParamError(format!("invalid uuid: {}", s)))
    }
}

impl PathParam for ReactionKeyParam {
    fn from_path_param(s: &str) -> Result<Self, PathParamError> {
        s.parse::<ReactionKeyParam>()
            .map_err(|_| PathParamError(format!("invalid reaction key: {}", s)))
    }
}

impl PathParam for DocumentRevisionId {
    fn from_path_param(s: &str) -> Result<Self, PathParamError> {
        s.parse()
            .map_err(|e: String| PathParamError(format!("invalid document revision id: {}", e)))
    }
}
