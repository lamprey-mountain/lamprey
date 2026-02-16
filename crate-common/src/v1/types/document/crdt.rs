//! crdt types

/// a pointer to a client's state at a point in time
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentStateVector(pub Vec<u8>);

/// an update to a document crdt
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentUpdate(pub Vec<u8>);

#[cfg(feature = "serde")]
mod serde_impl {
    use super::{DocumentStateVector, DocumentUpdate};
    use base64::{prelude::BASE64_URL_SAFE_NO_PAD, Engine};
    use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

    impl Serialize for DocumentStateVector {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            if serializer.is_human_readable() {
                serializer.serialize_str(&BASE64_URL_SAFE_NO_PAD.encode(&self.0))
            } else {
                serializer.serialize_bytes(&self.0)
            }
        }
    }

    impl<'de> Deserialize<'de> for DocumentStateVector {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            if deserializer.is_human_readable() {
                let s = String::deserialize(deserializer)?;
                let bytes = BASE64_URL_SAFE_NO_PAD
                    .decode(s)
                    .map_err(de::Error::custom)?;
                Ok(DocumentStateVector(bytes))
            } else {
                let bytes = Vec::<u8>::deserialize(deserializer)?;
                Ok(DocumentStateVector(bytes))
            }
        }
    }

    impl Serialize for DocumentUpdate {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            if serializer.is_human_readable() {
                serializer.serialize_str(&BASE64_URL_SAFE_NO_PAD.encode(&self.0))
            } else {
                serializer.serialize_bytes(&self.0)
            }
        }
    }

    impl<'de> Deserialize<'de> for DocumentUpdate {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            if deserializer.is_human_readable() {
                let s = String::deserialize(deserializer)?;
                let bytes = BASE64_URL_SAFE_NO_PAD
                    .decode(s)
                    .map_err(de::Error::custom)?;
                Ok(DocumentUpdate(bytes))
            } else {
                let bytes = Vec::<u8>::deserialize(deserializer)?;
                Ok(DocumentUpdate(bytes))
            }
        }
    }
}

#[cfg(feature = "utoipa")]
mod utoipa_impl {
    use super::{DocumentStateVector, DocumentUpdate};
    use utoipa::{openapi::ObjectBuilder, PartialSchema, ToSchema};

    impl ToSchema for DocumentStateVector {}
    impl PartialSchema for DocumentStateVector {
        fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
            ObjectBuilder::new()
                .schema_type(utoipa::openapi::schema::Type::String)
                .description(Some("Base64 encoded state vector"))
                .build()
                .into()
        }
    }

    impl ToSchema for DocumentUpdate {}
    impl PartialSchema for DocumentUpdate {
        fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
            ObjectBuilder::new()
                .schema_type(utoipa::openapi::schema::Type::String)
                .description(Some("Base64 encoded update"))
                .build()
                .into()
        }
    }
}
