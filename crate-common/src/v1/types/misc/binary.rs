use std::ops::Deref;

use bytes::Bytes;

/// some binary data
///
/// serialized as unpaddeded url safe base64 for human readable formats (json)
/// and raw binary otherwise (msgpack)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binary<const MAX_SIZE: usize>(pub Bytes);

// TODO: struct BinaryUnlimited for unrestricted max size?
// TODO: rename MAX_SIZE to MAX_LEN or MAX_LENGTH?

#[cfg(feature = "utoipa")]
mod _u {
    use utoipa::{
        PartialSchema, ToSchema,
        openapi::{RefOr, Schema, schema::AnyOf},
        schema,
    };

    use crate::v1::types::misc::binary::Binary;

    // TODO: indicate MAX_SIZE in schema?
    impl<const MAX_SIZE: usize> PartialSchema for Binary<MAX_SIZE> {
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

    impl<const MAX_SIZE: usize> ToSchema for Binary<MAX_SIZE> {}
}

#[cfg(feature = "serde")]
mod _s {
    use core::fmt;

    use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
    use bytes::Bytes;
    use serde::{Deserialize, Serialize, de};

    use crate::v1::types::misc::binary::Binary;

    impl<const MAX_SIZE: usize> Serialize for Binary<MAX_SIZE> {
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

    impl<'de, const MAX_SIZE: usize> Deserialize<'de> for Binary<MAX_SIZE> {
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

            if bytes.len() > MAX_SIZE {
                // TODO: use api error
                Err(de::Error::custom(format!(
                    "length {} exceeds MAX_SIZE of {}",
                    bytes.len(),
                    MAX_SIZE
                )))
            } else {
                Ok(Binary(bytes))
            }
        }
    }
}

impl<const MAX_SIZE: usize> From<Binary<MAX_SIZE>> for Bytes {
    fn from(b: Binary<MAX_SIZE>) -> Self {
        b.0
    }
}

impl<const MAX_SIZE: usize> Deref for Binary<MAX_SIZE> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl<const MAX_SIZE: usize> AsRef<[u8]> for Binary<MAX_SIZE> {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

// TODO: impl these
// impl<const MAX_SIZE: usize> Binary<MAX_SIZE> {
//     pub fn new(v: Vec<u8>) -> ApiResult<Self> {
//         todo!()
//     }

//     // is unsafe correct here?
//     pub unsafe fn new_unchecked(v: Vec<u8>) -> Self {
//         todo!()
//     }
// }
