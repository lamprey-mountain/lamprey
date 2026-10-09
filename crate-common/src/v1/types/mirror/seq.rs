use std::{fmt, marker::PhantomData};

mod private {
    pub trait Sealed {}
}

pub trait Marker: private::Sealed {
    fn name() -> &'static str;
}

/// A monotonic sync token, incremented on every action in a context.
///
/// Used for incremental sync to determine what events the client is missing.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Seq<M: Marker> {
    inner: u64,
    _phantom: PhantomData<M>,
}

macro_rules! impl_seq {
    ($name:ident) => {
        pastey::paste! {
            #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            #[non_exhaustive]
            pub enum [<Marker $name>] {}

            impl Marker for [<Marker $name>] {
                fn name() -> &'static str {
                    stringify!($name)
                }
            }

            impl private::Sealed for [<Marker $name>] {}

            pub type [<$name Seq>] = Seq<[<Marker $name>]>;
        }
    };
}

impl<M: Marker> Seq<M> {
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.inner == 0
    }
}

impl<M: Marker> Default for Seq<M> {
    fn default() -> Self {
        0.into()
    }
}

impl<M: Marker> fmt::Display for Seq<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl<M: Marker> fmt::Debug for Seq<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}Seq({})", M::name(), self.inner)
    }
}

impl<M: Marker> From<u64> for Seq<M> {
    fn from(value: u64) -> Self {
        Self {
            inner: value,
            _phantom: PhantomData,
        }
    }
}

impl<M: Marker> From<Seq<M>> for u64 {
    fn from(value: Seq<M>) -> Self {
        value.inner
    }
}

#[cfg(feature = "serde")]
impl<M: Marker> serde::Serialize for Seq<M> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u64(self.inner)
    }
}

#[cfg(feature = "serde")]
impl<'de, M: Marker> serde::Deserialize<'de> for Seq<M> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(u64::deserialize(deserializer)?.into())
    }
}

#[cfg(feature = "utoipa")]
mod schema {
    use utoipa::{
        PartialSchema, ToSchema,
        openapi::{RefOr, schema::Schema},
        schema,
    };

    use super::{Marker, Seq};

    impl<M: Marker> PartialSchema for Seq<M> {
        fn schema() -> utoipa::openapi::RefOr<Schema> {
            RefOr::T(Schema::Object(
                schema!(u64)
                    .title(Some("Seq"))
                    .description(Some(
                        "A monotonic sync token, incremented on every action in a context.",
                    ))
                    .build(),
            ))
        }
    }

    impl<M: Marker> ToSchema for Seq<M> {}
}

impl_seq!(Channel);
impl_seq!(Room);
impl_seq!(User); // public user change
impl_seq!(Client); // private user change
