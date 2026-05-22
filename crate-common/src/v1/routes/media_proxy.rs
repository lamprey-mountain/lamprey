use lamprey_macros::endpoint;

/// Head media
///
/// get headers for a piece of media
#[endpoint(
    head,
    path = "/media/{media_id}",
    tags = ["media"],
    response(OK, description = "success"),
)]
pub mod media_head {
    use crate::{v1::types::MediaId, v2::types::media::proxy::MediaQuery};

    pub struct Request {
        #[path]
        pub media_id: MediaId,

        #[query]
        pub query: MediaQuery,
    }

    pub struct Response {}
}

/// Fetch media
///
/// download a piece of media
#[endpoint(
    get,
    path = "/media/{media_id}",
    tags = ["media"],
    response(OK, description = "success"),
)]
pub mod media_get {
    use crate::{v1::types::MediaId, v2::types::media::proxy::MediaQuery};

    pub struct Request {
        #[path]
        pub media_id: MediaId,

        #[query]
        pub query: MediaQuery,
    }

    pub struct Response {}
}

/// Head media with filename
///
/// get headers for a piece of media
#[endpoint(
    head,
    path = "/media/{media_id}/{filename}",
    tags = ["media"],
    response(OK, description = "success"),
)]
pub mod media_head_filename {
    use crate::{v1::types::MediaId, v2::types::media::proxy::MediaQuery};

    pub struct Request {
        #[path]
        pub media_id: MediaId,

        #[path]
        pub filename: String,

        #[query]
        pub query: MediaQuery,
    }

    pub struct Response {}
}

/// Fetch media with filename
///
/// download a piece of media
#[endpoint(
    get,
    path = "/media/{media_id}/{filename}",
    tags = ["media"],
    response(OK, description = "success"),
)]
pub mod media_get_filename {
    use crate::{v1::types::MediaId, v2::types::media::proxy::MediaQuery};

    pub struct Request {
        #[path]
        pub media_id: MediaId,

        #[path]
        pub filename: String,

        #[query]
        pub query: MediaQuery,
    }

    pub struct Response {}
}

/// Fetch thumbnail
///
/// get a thumbnail for a piece of media
#[endpoint(
    get,
    path = "/thumb/{media_id}",
    tags = ["media"],
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

    pub struct Response {}
}

/// Head thumbnail
///
/// get headers for a thumbnail for a piece of media
#[endpoint(
    head,
    path = "/thumb/{media_id}",
    tags = ["media"],
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

    pub struct Response {}
}

/// Fetch gifv
///
/// transcode a gif into a video
#[endpoint(
    get,
    path = "/gifv/{media_id}",
    tags = ["media"],
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

    pub struct Response {}
}

/// Head gifv
///
/// get headers for a transcoded gif
#[endpoint(
    head,
    path = "/gifv/{media_id}",
    tags = ["media"],
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

    pub struct Response {}
}

/// Fetch emoji
///
/// directly get an emoji's thumbnail
#[endpoint(
    get,
    path = "/emoji/{emoji_id}",
    tags = ["media"],
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

    pub struct Response {}
}

/// Head emoji
///
/// directly get an emoji's thumbnail headers
#[endpoint(
    head,
    path = "/emoji/{emoji_id}",
    tags = ["media"],
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

    pub struct Response {}
}

/// Fetch trickplay
#[endpoint(
    get,
    path = "/trickplay/{media_id}",
    tags = ["media"],
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

    pub struct Response {}
}

/// Head trickplay
#[endpoint(
    head,
    path = "/trickplay/{media_id}",
    tags = ["media"],
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

    pub struct Response {}
}

/// Fetch stream
#[endpoint(
    get,
    path = "/stream/{media_id}",
    tags = ["media"],
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

    pub struct Response {}
}

/// Head stream
#[endpoint(
    head,
    path = "/stream/{media_id}",
    tags = ["media"],
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

    pub struct Response {}
}
