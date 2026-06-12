use lamprey_macros::endpoint;

/// Media create
#[endpoint(
    post,
    path = "/media",
    tags = ["media"],
    response(CREATED, body = MediaCreated, description = "Media create success"),
)]
pub mod media_create {
    use crate::v2::types::media::{MediaCreate, MediaCreated};

    pub struct Request {
        #[json]
        pub body: MediaCreate,
    }

    pub struct Response {
        #[json]
        pub media: MediaCreated,
    }
}

/// Media get
#[endpoint(
    get,
    path = "/media/{media_id}",
    tags = ["media"],
    response(OK, body = Media, description = "Media get success"),
)]
pub mod media_get {
    use crate::{v1::types::MediaId, v2::types::media::Media};

    pub struct Request {
        #[path]
        pub media_id: MediaId,
    }

    pub struct Response {
        #[json]
        pub media: Media,
    }
}

/// Media patch
#[endpoint(
    patch,
    path = "/media/{media_id}",
    tags = ["media"],
    response(OK, body = Media, description = "Media patch success"),
)]
pub mod media_patch {
    use crate::{
        v1::types::MediaId,
        v2::types::media::{Media, MediaPatch},
    };

    pub struct Request {
        #[path]
        pub media_id: MediaId,

        #[json]
        pub body: MediaPatch,
    }

    pub struct Response {
        #[json]
        pub media: Media,
    }
}

/// Media delete
#[endpoint(
    delete,
    path = "/media/{media_id}",
    tags = ["media"],
    response(NO_CONTENT, description = "Media delete success"),
    response(CONFLICT, description = "Media linked to another resource"),
)]
pub mod media_delete {
    use crate::v1::types::MediaId;

    pub struct Request {
        #[path]
        pub media_id: MediaId,
    }

    pub struct Response {}
}

/// Media done
#[endpoint(
    put,
    path = "/media/{media_id}/done",
    tags = ["media"],
    response(OK, body = Media, description = "Media processing finished"),
    response(ACCEPTED, description = "Media processing in background"),
)]
pub mod media_done {
    use crate::{
        v1::types::MediaId,
        v2::types::media::{Media, MediaDoneParams},
    };

    pub struct Request {
        #[path]
        pub media_id: MediaId,

        #[json]
        pub body: MediaDoneParams,
    }

    pub struct Response {
        // #[status]
        // pub status: StatusCode,
        #[json]
        pub media: Option<Media>,
    }
}

/// Media clone
#[endpoint(
    post,
    path = "/media/{media_id}/clone",
    tags = ["media"],
    response(OK, body = Media, description = "Media clone success"),
)]
pub mod media_clone {
    use crate::{
        v1::types::MediaId,
        v2::types::media::{Media, MediaClone},
    };

    pub struct Request {
        #[path]
        pub media_id: MediaId,

        #[json]
        pub body: MediaClone,
    }

    pub struct Response {
        #[json]
        pub media: Media,
    }
}

/// Media search
#[endpoint(
    post,
    path = "/media/search",
    tags = ["media"],
    response(OK, body = MediaSearch, description = "Media search success"),
)]
pub mod media_search {
    use crate::v1::types::search::{MediaSearch, MediaSearchRequest};

    pub struct Request {
        #[json]
        pub body: MediaSearchRequest,
    }

    pub struct Response {
        #[json]
        pub results: MediaSearch,
    }
}
