use std::{
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures::{TryStream, TryStreamExt};
use http_body::Frame;
use http_body_util::{BodyExt, Empty, StreamBody, combinators::UnsyncBoxBody};

// most of this is copied from axum's body impl

/// The body type used for lamprey endpoints.
#[must_use]
#[derive(Debug)]
pub struct Body(UnsyncBoxBody<Bytes, BodyError>);

pub type BodyError = Box<dyn core::error::Error + Send + Sync + 'static>;

impl Body {
    /// Create a new `Body` that wraps another [`http_body::Body`].
    pub fn new<B>(body: B) -> Self
    where
        B: http_body::Body<Data = Bytes> + Send + 'static,
        B::Error: Into<BodyError>,
    {
        Self(UnsyncBoxBody::new(
            body.map_err(|e| e.into()).map_frame(|data| data.into()),
        ))
    }

    /// Create an empty body.
    pub fn empty() -> Self {
        Self::new(Empty::new())
    }

    /// Create a new `Body` from a [`Stream`].
    ///
    /// [`Stream`]: https://docs.rs/futures-core/latest/futures_core/stream/trait.Stream.html
    pub fn from_stream<S>(stream: S) -> Self
    where
        S: TryStream + Send + 'static,
        S::Ok: Into<Bytes>,
        S::Error: Into<BodyError>,
    {
        let stream = stream
            .map_ok(|data| Frame::data(data.into()))
            .map_err(|e| e.into());
        Self::new(StreamBody::new(stream))
    }

    /// Collect the entire [`Body`] into [`Bytes`]
    pub async fn buffer(self) -> core::result::Result<Bytes, BodyError> {
        let collected = self.0.collect().await.map_err(Into::<BodyError>::into)?;
        Ok(collected.to_bytes())
    }
}

impl Default for Body {
    fn default() -> Self {
        Self::empty()
    }
}

macro_rules! body_from_impl {
    ($ty:ty) => {
        impl From<$ty> for Body {
            fn from(buf: $ty) -> Self {
                Self::new(http_body_util::Full::from(buf))
            }
        }
    };
}

body_from_impl!(&'static [u8]);
body_from_impl!(std::borrow::Cow<'static, [u8]>);
body_from_impl!(Vec<u8>);

body_from_impl!(&'static str);
body_from_impl!(std::borrow::Cow<'static, str>);
body_from_impl!(String);

body_from_impl!(Bytes);

impl http_body::Body for Body {
    type Data = Bytes;
    type Error = BodyError;

    #[inline]
    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        Pin::new(&mut self.0).poll_frame(cx)
    }

    #[inline]
    fn size_hint(&self) -> http_body::SizeHint {
        self.0.size_hint()
    }

    #[inline]
    fn is_end_stream(&self) -> bool {
        self.0.is_end_stream()
    }
}

// TODO: gate this behind serde feature
impl<'de> serde::Deserialize<'de> for Body {
    fn deserialize<D>(_de: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Err(serde::de::Error::custom(
            "Body is not deserializable; should_parse() should have prevented this",
        ))
    }
}
