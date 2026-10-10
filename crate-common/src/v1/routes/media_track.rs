use lamprey_macros::endpoint;
use uuid::Uuid;

use crate::{
    v1::types::MediaId,
    v2::types::media::track::{Track, TrackCreate, TrackUpdate},
};

/// Media track create
#[endpoint(
    post,
    path = "/media/{media_id}/track",
    tags = ["media"],
    response(CREATED, body = Track, description = "Track create success"),
)]
pub mod media_track_create {
    use super::*;

    pub struct Request {
        #[path]
        pub media_id: MediaId,

        #[json]
        pub body: TrackCreate,
    }

    pub struct Response {
        #[json]
        pub track: Track,
    }
}

/// Media track list
#[endpoint(
    get,
    path = "/media/{media_id}/track",
    tags = ["media"],
    response(OK, body = Vec<Track>, description = "Track list success"),
)]
pub mod media_track_list {
    use super::*;

    pub struct Request {
        #[path]
        pub media_id: MediaId,
    }

    pub struct Response {
        #[json]
        pub tracks: Vec<Track>,
    }
}

/// Media track update
#[endpoint(
    patch,
    path = "/media/{media_id}/track/{track_id}",
    tags = ["media"],
    response(OK, body = Track, description = "Track update success"),
)]
pub mod media_track_update {
    use super::*;

    pub struct Request {
        #[path]
        pub media_id: MediaId,

        #[path]
        pub track_id: Uuid,

        #[json]
        pub body: TrackUpdate,
    }

    pub struct Response {
        #[json]
        pub track: Track,
    }
}

/// Media track delete
#[endpoint(
    delete,
    path = "/media/{media_id}/track/{track_id}",
    tags = ["media"],
    response(NO_CONTENT, description = "Track delete success"),
)]
pub mod media_track_delete {
    use super::*;

    pub struct Request {
        #[path]
        pub media_id: MediaId,

        #[path]
        pub track_id: Uuid,
    }

    pub struct Response {}
}

/// Media track get
#[endpoint(
    get,
    path = "/media/{media_id}/track/{track_id}",
    tags = ["media"],
    response(OK, body = Track, description = "Track get success"),
)]
pub mod media_track_get {
    use super::*;

    pub struct Request {
        #[path]
        pub media_id: MediaId,

        #[path]
        pub track_id: Uuid,
    }

    pub struct Response {
        #[json]
        pub track: Track,
    }
}
