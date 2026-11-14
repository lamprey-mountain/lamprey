//! query string deserialization that supports repeated keys as sequences

use std::collections::HashMap;
use std::fmt::{self, Display};

use serde::de::value::MapDeserializer;
use serde::de::{
    self, DeserializeOwned, DeserializeSeed, Deserializer, IntoDeserializer, SeqAccess, Visitor,
};

/// error returned when deserializing query parameters fails
#[derive(Debug)]
pub struct QueryError(String);

/// all values supplied for a single query key
struct Values(Vec<String>);

/// the values of a single query key as a sequence
struct SeqValues(std::vec::IntoIter<String>);

macro_rules! forward_parsed_value {
    ($($ty:ident => $method:ident,)*) => {
        $(
            fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
                match self.0.as_slice() {
                    [value] => match value.parse::<$ty>() {
                        Ok(value) => value.into_deserializer().$method(visitor),
                        Err(e) => Err(de::Error::custom(e)),
                    },
                    values => Err(de::Error::invalid_length(values.len(), &"a single value")),
                }
            }
        )*
    };
}

impl Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for QueryError {}

impl de::Error for QueryError {
    fn custom<T: Display>(msg: T) -> Self {
        Self(msg.to_string())
    }
}

impl<'de> IntoDeserializer<'de, QueryError> for Values {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

impl<'de> Deserializer<'de> for Values {
    type Error = QueryError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.0.as_slice() {
            [value] => visitor.visit_string(value.clone()),
            values => Err(de::Error::invalid_length(values.len(), &"a single value")),
        }
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        if self.0.is_empty() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_seq(SeqValues(self.0.into_iter()))
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        match self.0.as_slice() {
            [value] => visitor.visit_enum(value.clone().into_deserializer()),
            values => Err(de::Error::invalid_length(values.len(), &"a single value")),
        }
    }

    serde::forward_to_deserialize_any! {
        char str string bytes byte_buf unit unit_struct tuple tuple_struct struct
        identifier ignored_any map
    }

    forward_parsed_value! {
        bool => deserialize_bool,
        u8 => deserialize_u8,
        u16 => deserialize_u16,
        u32 => deserialize_u32,
        u64 => deserialize_u64,
        i8 => deserialize_i8,
        i16 => deserialize_i16,
        i32 => deserialize_i32,
        i64 => deserialize_i64,
        f32 => deserialize_f32,
        f64 => deserialize_f64,
    }
}

impl<'de> SeqAccess<'de> for SeqValues {
    type Error = QueryError;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Self::Error> {
        match self.0.next() {
            Some(value) => seed.deserialize(value.into_deserializer()).map(Some),
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.0.len())
    }
}

/// deserialize `T` from a query string
///
/// unlike [`serde_urlencoded`], repeated keys are collected into a sequence,
/// so `channel_id=a&channel_id=b` deserializes into `Vec<ChannelId>`
pub fn from_str<T: DeserializeOwned>(input: &str) -> Result<T, QueryError> {
    let pairs: Vec<(String, String)> =
        serde_urlencoded::from_str(input).map_err(|e| QueryError(e.to_string()))?;

    let mut grouped: HashMap<String, Vec<String>> = HashMap::new();
    for (key, value) in pairs {
        grouped.entry(key).or_default().push(value);
    }

    T::deserialize(MapDeserializer::new(
        grouped
            .into_iter()
            .map(|(key, values)| (key, Values(values))),
    ))
}
