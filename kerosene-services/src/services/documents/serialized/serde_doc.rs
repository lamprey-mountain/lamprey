use serde::de::DeserializeOwned;
use yrs::Doc;

use crate::{prelude::*, services::documents::serialized::FromDoc};

impl<T: DeserializeOwned> FromDoc for T {
    type Error = Error;

    fn from_doc(doc: &Doc) -> CoreResult<Self, Self::Error> {
        struct DocDeserializer;

        #[derive(Debug, thiserror::Error)]
        struct DocDeserializerError;

        impl serde::de::Error for DocDeserializerError {}

        impl<'de> serde::Deserializer<'de> for DocDeserializer {
            type Error = DocDeserializerError;

            fn deserialize_any<V>(self, visitor: V) -> CoreResult<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                todo!()
            }

            fn deserialize_bool<V>(self, visitor: V) -> CoreResult<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                todo!()
            }

            fn deserialize_i8<V>(self, visitor: V) -> CoreResult<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                todo!()
            }

            fn deserialize_i16<V>(self, visitor: V) -> CoreResult<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                todo!()
            }

            fn deserialize_i32<V>(self, visitor: V) -> CoreResult<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                todo!()
            }

            fn deserialize_i64<V>(self, visitor: V) -> CoreResult<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                todo!()
            }

            fn deserialize_u8<V>(self, visitor: V) -> CoreResult<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                todo!()
            }

            fn deserialize_u16<V>(self, visitor: V) -> CoreResult<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                todo!()
            }

            fn deserialize_u32<V>(self, visitor: V) -> CoreResult<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                todo!()
            }

            fn deserialize_u64<V>(self, visitor: V) -> CoreResult<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                todo!()
            }

            fn deserialize_f32<V>(self, visitor: V) -> CoreResult<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                todo!()
            }

            fn deserialize_f64<V>(self, visitor: V) -> CoreResult<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                todo!()
            }

            fn deserialize_char<V>(self, visitor: V) -> CoreResult<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                todo!()
            }

            fn deserialize_str<V>(self, visitor: V) -> CoreResult<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                todo!()
            }

            fn deserialize_string<V>(self, visitor: V) -> CoreResult<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                todo!()
            }

            fn deserialize_bytes<V>(self, visitor: V) -> CoreResult<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                todo!()
            }

            fn deserialize_byte_buf<V>(self, visitor: V) -> CoreResult<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                todo!()
            }

            fn deserialize_option<V>(self, visitor: V) -> CoreResult<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                todo!()
            }

            fn deserialize_unit<V>(self, visitor: V) -> CoreResult<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                todo!()
            }

            fn deserialize_unit_struct<V>(
                self,
                name: &'static str,
                visitor: V,
            ) -> CoreResult<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                todo!()
            }

            fn deserialize_newtype_struct<V>(
                self,
                name: &'static str,
                visitor: V,
            ) -> CoreResult<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                todo!()
            }

            fn deserialize_seq<V>(self, visitor: V) -> CoreResult<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                todo!()
            }

            fn deserialize_tuple<V>(
                self,
                len: usize,
                visitor: V,
            ) -> CoreResult<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                todo!()
            }

            fn deserialize_tuple_struct<V>(
                self,
                name: &'static str,
                len: usize,
                visitor: V,
            ) -> CoreResult<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                todo!()
            }

            fn deserialize_map<V>(self, visitor: V) -> CoreResult<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                todo!()
            }

            fn deserialize_struct<V>(
                self,
                name: &'static str,
                fields: &'static [&'static str],
                visitor: V,
            ) -> CoreResult<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                todo!()
            }

            fn deserialize_enum<V>(
                self,
                name: &'static str,
                variants: &'static [&'static str],
                visitor: V,
            ) -> CoreResult<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                todo!()
            }

            fn deserialize_identifier<V>(self, visitor: V) -> CoreResult<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                todo!()
            }

            fn deserialize_ignored_any<V>(self, visitor: V) -> CoreResult<V::Value, Self::Error>
            where
                V: serde::de::Visitor<'de>,
            {
                todo!()
            }
        }

        Self::deserialize(DocDeserializer)
    }

    fn from_doc_lenient(doc: &Doc) -> CoreResult<Self, Self::Error> {
        todo!()
    }
}
