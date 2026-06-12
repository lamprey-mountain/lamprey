use std::collections::HashMap;
use std::convert::Infallible;
use std::str::FromStr;

use crate::prelude::*;
use crate::routes::util::extract::{parse_json, parse_msgpack, ExtractorError};
use multer::Multipart;
use serde::de::DeserializeOwned;
use serde_json::Value;

/// the parsed name of a multipart field
pub enum MultipartFieldName {
    /// embedded json (`payload_json`)
    PayloadJson,

    /// embedded msgpack (`payload_msgpack`)
    PayloadMsgpack,

    /// media to upload, (`media[n]`)
    Media(u64),

    /// any other field (`anything_else`)
    ///
    /// parsed as json and merged into payload
    Field(String),
}

/// a file uploaded via multipart/form-data
#[derive(Debug)]
pub struct MultipartFile {
    pub filename: Option<String>,
    pub content_type: Option<String>,
    pub data: Bytes,
}

pub type MultipartFiles = HashMap<u64, MultipartFile>;

/// utility to parse a multipart body
// NOTE: Value may be incorrect here for msgpack?
#[derive(Debug, Default)]
pub struct MultipartCollector {
    has_payload: bool,
    payload: Option<HashMap<String, Value>>,
    fields: HashMap<String, Value>,
    media: HashMap<u64, MultipartFile>,
}

impl FromStr for MultipartFieldName {
    type Err = Infallible;

    fn from_str(s: &str) -> CoreResult<Self, Infallible> {
        if s == "payload_json" {
            return Ok(MultipartFieldName::PayloadJson);
        }
        if s == "payload_msgpack" {
            return Ok(MultipartFieldName::PayloadMsgpack);
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

impl MultipartCollector {
    pub async fn collect(mut multipart: Multipart<'_>) -> Result<Self> {
        let mut me = Self::default();
        let mut errors = vec![];

        while let Some(field) = multipart.next_field().await? {
            if let Err(err) = me.handle(field).await {
                errors.push(err);
            }
        }

        if errors.is_empty() {
            Ok(me)
        } else {
            // TODO: return all errors
            Err(errors.into_iter().next().unwrap())
        }
    }

    pub fn parse<T: DeserializeOwned>(self) -> Result<(T, MultipartFiles)> {
        let mut payload = match self.payload {
            Some(mut p) => {
                for (key, value) in self.fields {
                    if p.insert(key, value).is_some() {
                        return Err(ExtractorError::MultipartDuplicateField.into());
                    }
                }

                p
            }
            None if !self.fields.is_empty() => self.fields,
            None => return Err(ExtractorError::MissingBody.into()),
        };

        let body = serde_json::to_value(payload).expect("always serializable?");
        let body = serde_json::from_value(body).map_err(|err| {
            Error::BadRequest(format!("failed to deserialize multipart payload: {err}"))
        })?;

        Ok((body, self.media))
    }

    pub async fn into_files(self) -> Result<MultipartFiles> {
        Ok(self.media)
    }

    async fn handle(&mut self, field: multer::Field<'_>) -> Result<()> {
        let Ok(name) = field
            .name()
            .ok_or(ExtractorError::MultipartNamelessField)?
            .parse::<MultipartFieldName>();

        let content_type = field.content_type().map(|s| s.to_owned());
        let file_name = field.file_name().map(|s| s.to_owned());

        let bytes = field.bytes().await?;
        match name {
            MultipartFieldName::PayloadJson => {
                self.add_payload(parse_json(&bytes)?);
            }
            MultipartFieldName::PayloadMsgpack => {
                self.add_payload(parse_msgpack(&bytes)?);
            }
            MultipartFieldName::Media(n) => {
                let file = MultipartFile {
                    filename: file_name,
                    content_type: content_type.map(|c| c.to_string()),
                    data: bytes,
                };
                if self.media.insert(n, file).is_some() {
                    return Err(ExtractorError::MultipartDuplicateMedia.into());
                }
            }
            MultipartFieldName::Field(name) => {
                let json: Value = parse_json(&bytes)?;
                self.add_field(name, json)?;
            }
        }

        Ok(())
    }

    fn add_payload(&mut self, payload: HashMap<String, Value>) -> CoreResult<(), ExtractorError> {
        self.has_payload = true;
        if self.payload.is_some() {
            Err(ExtractorError::MultipartDuplicatePayload)
        } else {
            self.payload = Some(payload);
            Ok(())
        }
    }

    fn add_field(&mut self, name: String, value: Value) -> CoreResult<(), ExtractorError> {
        if self.fields.contains_key(&name) {
            Err(ExtractorError::MultipartDuplicateField)
        } else {
            self.fields.insert(name, value);
            Ok(())
        }
    }
}
