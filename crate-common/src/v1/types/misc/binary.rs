use std::ops::Deref;

use crate::v1::types::error::{ApiError, ApiResult, ErrorCode};
use bytes::Bytes;

/// some binary data
///
/// serialized as unpaddeded url safe base64 for human readable formats (json)
/// and raw binary otherwise (msgpack)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Binary<const MAX_LENGTH: usize>(pub Bytes);

// TODO: struct BinaryUnlimited for unrestricted max size?

impl<const MAX_LENGTH: usize> Binary<MAX_LENGTH> {
    pub fn new(v: Vec<u8>) -> ApiResult<Self> {
        if v.len() > MAX_LENGTH {
            return Err(ApiError::with_message(
                ErrorCode::InvalidData,
                format!("length {} exceeds max length of {}", v.len(), MAX_LENGTH),
            ));
        }
        Ok(Self(v.into()))
    }

    /// create a new binary without checking the length
    ///
    /// # Safety
    ///
    /// this does not cause undefined behavior, but it may violate the domain invariant
    /// that the length is less than or equal to MAX_LENGTH
    pub unsafe fn new_unchecked(v: Vec<u8>) -> Self {
        Self(v.into())
    }
}

#[cfg(feature = "utoipa")]
mod _u {
    use utoipa::{
        PartialSchema, ToSchema,
        openapi::{RefOr, Schema, schema::AnyOf},
        schema,
    };

    use crate::v1::types::misc::binary::Binary;

    // TODO: indicate MAX_LENGTH in schema?
    // .description(format!("binary data (max len {})", MAX_LENGTH)),
    impl<const MAX_LENGTH: usize> PartialSchema for Binary<MAX_LENGTH> {
        fn schema() -> RefOr<Schema> {
            RefOr::T(
                AnyOf::builder()
                    .item(schema!(#[inline] Vec<u8>).description(Some("raw bytes")))
                    .item(
                        schema!(
                            #[inline]
                            String
                        )
                        .description(Some("unpadded url safe base64")),
                    )
                    .description(Some("binary data"))
                    .build()
                    .into(),
            )
        }
    }

    impl<const MAX_LENGTH: usize> ToSchema for Binary<MAX_LENGTH> {}
}

#[cfg(feature = "serde")]
mod _s {
    use core::fmt;

    use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
    use bytes::Bytes;
    use serde::{Deserialize, Serialize, de};

    use crate::v1::types::misc::binary::Binary;

    impl<const MAX_LENGTH: usize> Serialize for Binary<MAX_LENGTH> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            if serializer.is_human_readable() {
                let encoded = BASE64_URL_SAFE_NO_PAD.encode(&self.0);
                serializer.serialize_str(&encoded)
            } else {
                serializer.serialize_bytes(&self.0)
            }
        }
    }

    impl<'de, const MAX_LENGTH: usize> Deserialize<'de> for Binary<MAX_LENGTH> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let bytes: Bytes = if deserializer.is_human_readable() {
                let s = String::deserialize(deserializer)?;
                let bytes = BASE64_URL_SAFE_NO_PAD
                    .decode(s.as_bytes())
                    .map_err(de::Error::custom)?;
                bytes.into()
            } else {
                struct BinaryVisitor;

                impl<'de> de::Visitor<'de> for BinaryVisitor {
                    type Value = Bytes;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("byte array")
                    }

                    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        Ok(Bytes::copy_from_slice(v))
                    }

                    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        Ok(Bytes::from(v))
                    }
                }

                deserializer.deserialize_byte_buf(BinaryVisitor)?
            };

            if bytes.len() > MAX_LENGTH {
                // TODO: use api error
                Err(de::Error::custom(format!(
                    "length {} exceeds max length of {}",
                    bytes.len(),
                    MAX_LENGTH
                )))
            } else {
                Ok(Binary(bytes))
            }
        }
    }
}

impl<const MAX_LENGTH: usize> From<Binary<MAX_LENGTH>> for Bytes {
    fn from(b: Binary<MAX_LENGTH>) -> Self {
        b.0
    }
}

impl<const MAX_LENGTH: usize> From<Bytes> for Binary<MAX_LENGTH> {
    fn from(b: Bytes) -> Self {
        Self(b)
    }
}

impl<const MAX_LENGTH: usize> From<Vec<u8>> for Binary<MAX_LENGTH> {
    fn from(b: Vec<u8>) -> Self {
        Self(b.into())
    }
}

impl<const MAX_LENGTH: usize> Deref for Binary<MAX_LENGTH> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl<const MAX_LENGTH: usize> AsRef<[u8]> for Binary<MAX_LENGTH> {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}
