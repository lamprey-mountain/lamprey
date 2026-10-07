use lamprey_macros::record;

pub mod webhook;
pub mod websocket;
pub mod webtransport;

// NOTE: i thought that putting the api version in the path would be better,
// but apparently websockets are hard to load balance. being able to use
// arbitrary urls/paths in the future could be helpful.
// NOTE: maybe this version struct should be moved so some sort of `mod unversioned`?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[repr(u8)]
pub enum Version {
    /// multiplexes everything over a single websocket
    // #[deprecated]
    V1 = 1,

    /// stream based sync
    V2 = 2,
}

/// how data should be compressed
#[record]
#[derive(Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Compression {
    /// Deflate compression
    Deflate,
}

/// how data should be encoded
#[record]
#[derive(Default, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Encoding {
    #[default]
    Json,
    Msgpack,
}

// /// configuration for a connection
// #[record(params)]
// pub struct Config {
//     pub version: Version,
//
//     pub compression: Option<Compression>,
//
//     #[serde(default)]
//     pub encoding: Encoding,
// }

#[cfg(feature = "serde")]
impl serde::Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u8(*self as u8)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match u8::deserialize(deserializer)? {
            1 => Ok(Version::V1),
            2 => Ok(Version::V2),
            n => Err(serde::de::Error::unknown_variant(&n.to_string(), &["1"])),
        }
    }
}
