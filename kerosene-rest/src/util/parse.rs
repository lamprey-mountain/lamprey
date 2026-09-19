use common::v1::types::error::{ErrorField, ErrorFieldType};
use serde::de::DeserializeOwned;

use crate::prelude::*;

pub fn parse_json<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
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

pub fn parse_form<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
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

pub fn parse_msgpack<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let data: T = rmp_serde::from_slice(bytes).map_err(|err| ApiError {
        message: err.to_string(),
        fields: vec![],
        ..ApiError::from_code(ErrorCode::InvalidData)
    })?;
    Ok(data)
}
