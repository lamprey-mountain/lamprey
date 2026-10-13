use anyhow::{Context, Result};
use common::v2::types::{ChannelId, MediaId, MessageId, RoleId, RoomId, UserId};

pub const KEY_SEP: u8 = 0xFF;

pub trait Key: Sized {
    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Result<Self>;
}

impl Key for String {
    fn to_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        std::str::from_utf8(bytes)
            .map(str::to_owned)
            .context("invalid utf-8 in key")
    }
}

macro_rules! impl_key_uuid {
($($t:ty),*) => {
    $(
        impl Key for $t {
            // use the string representation so a uuid can never contain KEY_SEP (0xFF)
            fn to_bytes(&self) -> Vec<u8> {
                self.to_string().into_bytes()
            }

            fn from_bytes(bytes: &[u8]) -> Result<Self> {
                std::str::from_utf8(bytes)
                    .context("invalid utf-8 in uuid key")?
                    .parse()
                    .context("invalid uuid in key")
            }
        }
    )*
};
}

macro_rules! impl_key_tuple {
($first:ident $(, $rest:ident)*) => {
    #[allow(non_snake_case)]
    impl<$first: Key $(, $rest: Key)*> Key for ($first, $($rest),*) {
        fn to_bytes(&self) -> Vec<u8> {
            let ($first, $($rest),*) = self;
            #[allow(unused_mut)] // warning with tuple with one element
            let mut out = $first.to_bytes();
            $(
                out.push(KEY_SEP);
                out.extend_from_slice(&$rest.to_bytes());
            )*
            out
        }

        fn from_bytes(bytes: &[u8]) -> Result<Self> {
            let mut parts = bytes.split(|&b| b == KEY_SEP);
            let $first = $first::from_bytes(
                parts.next().context("missing tuple key segment")?
            )?;
            $(
                let $rest = $rest::from_bytes(
                    parts.next().context("missing tuple key segment")?
                )?;
            )*
            if parts.next().is_some() {
                anyhow::bail!("too many segments in tuple key");
            }
            Ok(($first, $($rest),*))
        }
    }
};
}

impl_key_uuid!(UserId, ChannelId, RoomId, MessageId, MediaId, RoleId);

impl_key_tuple!(A);
impl_key_tuple!(A, B);
impl_key_tuple!(A, B, C);
impl_key_tuple!(A, B, C, D);
impl_key_tuple!(A, B, C, D, E);
impl_key_tuple!(A, B, C, D, E, F);
