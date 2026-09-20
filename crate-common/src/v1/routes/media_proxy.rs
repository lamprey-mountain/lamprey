use lamprey_macros::endpoint;

// TODO: remove the HEAD routes as they're already covered by GET.
// i'll need to return a proper http body stream instead of bytes, otherwise i'd have to read the entire file on HEAD (among other things, like buffering the entire response body in memory)
// i'd probably need to add a #[method] attr to get the request method.

/// Head media
///
/// get headers for a piece of media
#[endpoint(
    head,
    path = "/media/{media_id}",
    tags = ["cdn"],
    response(OK, description = "success"),
)]
pub mod media_head {
    use http::{HeaderMap, StatusCode};

    use crate::{v1::types::MediaId, v2::types::media::proxy::MediaQuery};

    pub struct Request {
        #[path]
        pub media_id: MediaId,

        #[query]
        pub query: MediaQuery,
    }

    pub struct Response {
        #[status]
        pub status: StatusCode,

        #[headers]
        pub headers: HeaderMap,
    }
}

/// Fetch media
///
/// download a piece of media
#[endpoint(
    get,
    path = "/media/{media_id}",
    tags = ["cdn"],
    response(OK, description = "success"),
)]
pub mod media_get {
    use bytes::Bytes;
    use http::{HeaderMap, StatusCode};

    use crate::{util::body::Body, v1::types::MediaId, v2::types::media::proxy::MediaQuery};

    pub struct Request {
        #[path]
        pub media_id: MediaId,

        #[query]
        pub query: MediaQuery,
    }

    pub struct Response {
        #[status]
        pub status: StatusCode,

        #[headers]
        pub headers: HeaderMap,

        #[body]
        pub body: Body,
    }
}

/// Head media with filename
///
/// get headers for a piece of media
#[endpoint(
    head,
    path = "/media/{media_id}/{filename}",
    tags = ["cdn"],
    response(OK, description = "success"),
)]
pub mod media_head_filename {
    use http::{HeaderMap, StatusCode};

    use crate::{v1::types::MediaId, v2::types::media::proxy::MediaQuery};

    pub struct Request {
        #[path]
        pub media_id: MediaId,

        #[path]
        pub filename: String,

        #[query]
        pub query: MediaQuery,
    }

    pub struct Response {
        #[status]
        pub status: StatusCode,

        #[headers]
        pub headers: HeaderMap,
    }
}

/// Fetch media with filename
///
/// download a piece of media
#[endpoint(
    get,
    path = "/media/{media_id}/{filename}",
    tags = ["cdn"],
    response(OK, description = "success"),
)]
pub mod media_get_filename {
    use bytes::Bytes;
    use http::{HeaderMap, StatusCode};

    use crate::{util::body::Body, v1::types::MediaId, v2::types::media::proxy::MediaQuery};

    pub struct Request {
        #[path]
        pub media_id: MediaId,

        #[path]
        pub filename: String,

        #[query]
        pub query: MediaQuery,
    }

    pub struct Response {
        #[status]
        pub status: StatusCode,

        #[headers]
        pub headers: HeaderMap,

        #[body]
        pub body: Body,
    }
}

/// Fetch thumbnail
///
/// get a thumbnail for a piece of media
#[endpoint(
    get,
    path = "/thumb/{media_id}",
    tags = ["cdn"],
    response(OK, description = "success"),
)]
pub mod thumb_get {
    use crate::{
        v1::types::MediaId,
        v2::types::media::proxy::{MediaQuery, ThumbQuery},
    };

    pub struct Request {
        #[path]
        pub media_id: MediaId,

        #[query]
        pub query: ThumbQuery,

        #[query]
        pub media_query: MediaQuery,
    }

    pub struct Response {
        #[headers]
        pub headers: http::HeaderMap,
    }
}

/// Head thumbnail
///
/// get headers for a thumbnail for a piece of media
#[endpoint(
    head,
    path = "/thumb/{media_id}",
    tags = ["cdn"],
    response(OK, description = "success"),
)]
pub mod thumb_head {
    use crate::{
        v1::types::MediaId,
        v2::types::media::proxy::{MediaQuery, ThumbQuery},
    };

    pub struct Request {
        #[path]
        pub media_id: MediaId,

        #[query]
        pub query: ThumbQuery,

        #[query]
        pub media_query: MediaQuery,
    }

    pub struct Response {
        #[headers]
        pub headers: http::HeaderMap,
    }
}

/// Fetch gifv
///
/// transcode a gif into a video
#[endpoint(
    get,
    path = "/gifv/{media_id}",
    tags = ["cdn"],
    response(OK, description = "success"),
)]
pub mod gifv_get {
    use crate::{v1::types::MediaId, v2::types::media::proxy::MediaQuery};

    pub struct Request {
        #[path]
        pub media_id: MediaId,

        #[query]
        pub query: MediaQuery,
    }

    pub struct Response {
        #[headers]
        pub headers: http::HeaderMap,
    }
}

/// Head gifv
///
/// get headers for a transcoded gif
#[endpoint(
    head,
    path = "/gifv/{media_id}",
    tags = ["cdn"],
    response(OK, description = "success"),
)]
pub mod gifv_head {
    use crate::{v1::types::MediaId, v2::types::media::proxy::MediaQuery};

    pub struct Request {
        #[path]
        pub media_id: MediaId,

        #[query]
        pub query: MediaQuery,
    }

    pub struct Response {
        #[headers]
        pub headers: http::HeaderMap,
    }
}

/// Fetch emoji
///
/// directly get an emoji's thumbnail
#[endpoint(
    get,
    path = "/emoji/{emoji_id}",
    tags = ["cdn"],
    response(OK, description = "success"),
)]
pub mod emoji_get {
    use crate::{
        v1::types::EmojiId,
        v2::types::media::proxy::{MediaQuery, ThumbQuery},
    };

    pub struct Request {
        #[path]
        pub emoji_id: EmojiId,

        #[query]
        pub query: ThumbQuery,

        #[query]
        pub media_query: MediaQuery,
    }

    pub struct Response {
        #[headers]
        pub headers: http::HeaderMap,
    }
}

/// Head emoji
///
/// directly get an emoji's thumbnail headers
#[endpoint(
    head,
    path = "/emoji/{emoji_id}",
    tags = ["cdn"],
    response(OK, description = "success"),
)]
pub mod emoji_head {
    use crate::{
        v1::types::EmojiId,
        v2::types::media::proxy::{MediaQuery, ThumbQuery},
    };

    pub struct Request {
        #[path]
        pub emoji_id: EmojiId,

        #[query]
        pub query: ThumbQuery,

        #[query]
        pub media_query: MediaQuery,
    }

    pub struct Response {
        #[headers]
        pub headers: http::HeaderMap,
    }
}

/// Fetch trickplay
#[endpoint(
    get,
    path = "/trickplay/{media_id}",
    tags = ["cdn"],
    response(OK, description = "success"),
)]
pub mod trickplay_get {
    use crate::{
        v1::types::MediaId,
        v2::types::media::proxy::{MediaQuery, TrickplayQuery},
    };

    pub struct Request {
        #[path]
        pub media_id: MediaId,

        #[query]
        pub query: TrickplayQuery,

        #[query]
        pub media_query: MediaQuery,
    }

    pub struct Response {
        #[headers]
        pub headers: http::HeaderMap,
    }
}

/// Head trickplay
#[endpoint(
    head,
    path = "/trickplay/{media_id}",
    tags = ["cdn"],
    response(OK, description = "success"),
)]
pub mod trickplay_head {
    use crate::{
        v1::types::MediaId,
        v2::types::media::proxy::{MediaQuery, TrickplayQuery},
    };

    pub struct Request {
        #[path]
        pub media_id: MediaId,

        #[query]
        pub query: TrickplayQuery,

        #[query]
        pub media_query: MediaQuery,
    }

    pub struct Response {
        #[headers]
        pub headers: http::HeaderMap,
    }
}

/// Fetch stream
#[endpoint(
    get,
    path = "/stream/{media_id}",
    tags = ["cdn"],
    response(OK, description = "success"),
)]
pub mod stream_get {
    use crate::{
        v1::types::MediaId,
        v2::types::media::proxy::{MediaQuery, StreamQuery},
    };

    pub struct Request {
        #[path]
        pub media_id: MediaId,

        #[query]
        pub query: StreamQuery,

        #[query]
        pub media_query: MediaQuery,
    }

    pub struct Response {
        #[headers]
        pub headers: http::HeaderMap,
    }
}

/// Head stream
#[endpoint(
    head,
    path = "/stream/{media_id}",
    tags = ["cdn"],
    response(OK, description = "success"),
)]
pub mod stream_head {
    use crate::{
        v1::types::MediaId,
        v2::types::media::proxy::{MediaQuery, StreamQuery},
    };

    pub struct Request {
        #[path]
        pub media_id: MediaId,

        #[query]
        pub query: StreamQuery,

        #[query]
        pub media_query: MediaQuery,
    }

    pub struct Response {
        #[headers]
        pub headers: http::HeaderMap,
    }
}
