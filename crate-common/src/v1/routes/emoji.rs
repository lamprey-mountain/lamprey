use lamprey_macros::endpoint;

/// Emoji create
///
/// Create a custom emoji.
#[endpoint(
    post,
    path = "/room/{room_id}/emoji",
    tags = ["emoji"],
    scopes = [Full],
    permissions = [EmojiManage],
    audit_log_events = ["EmojiCreate"],
    response(CREATED, body = EmojiCustom, description = "new emoji created"),
)]
pub mod emoji_create {
    use crate::v1::types::emoji::{EmojiCustom, EmojiCustomCreate};
    use crate::v1::types::RoomId;

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[json]
        pub emoji: EmojiCustomCreate,

        #[header]
        pub idempotency_key: Option<String>,
    }

    pub struct Response {
        #[json]
        pub emoji: EmojiCustom,
    }
}

/// Emoji get
///
/// Get a custom emoji.
#[endpoint(
    get,
    path = "/room/{room_id}/emoji/{emoji_id}",
    tags = ["emoji"],
    scopes = [Full],
    response(OK, body = EmojiCustom, description = "success"),
)]
pub mod emoji_get {
    use crate::v1::types::emoji::EmojiCustom;
    use crate::v1::types::{EmojiId, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[path]
        pub emoji_id: EmojiId,
    }

    pub struct Response {
        #[json]
        pub emoji: EmojiCustom,
    }
}

/// Emoji delete
///
/// Delete a custom emoji.
#[endpoint(
    delete,
    path = "/room/{room_id}/emoji/{emoji_id}",
    tags = ["emoji"],
    scopes = [Full],
    permissions = [EmojiManage],
    audit_log_events = ["EmojiDelete"],
    response(NO_CONTENT, description = "success"),
)]
pub mod emoji_delete {
    use crate::v1::types::{EmojiId, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[path]
        pub emoji_id: EmojiId,
    }

    pub struct Response {}
}

/// Emoji update
///
/// Edit a custom emoji.
#[endpoint(
    patch,
    path = "/room/{room_id}/emoji/{emoji_id}",
    tags = ["emoji"],
    scopes = [Full],
    permissions = [EmojiManage],
    audit_log_events = ["EmojiUpdate"],
    response(OK, body = EmojiCustom, description = "success"),
)]
pub mod emoji_update {
    use crate::v1::types::emoji::{EmojiCustom, EmojiCustomPatch};
    use crate::v1::types::{EmojiId, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[path]
        pub emoji_id: EmojiId,

        #[json]
        pub patch: EmojiCustomPatch,
    }

    pub struct Response {
        #[json]
        pub emoji: EmojiCustom,
    }
}

/// Emoji list
///
/// List custom emoji in a room.
#[endpoint(
    get,
    path = "/room/{room_id}/emoji",
    tags = ["emoji"],
    scopes = [Full],
    response(OK, body = PaginationResponse<EmojiCustom>, description = "success"),
)]
pub mod emoji_list {
    use crate::v1::types::emoji::EmojiCustom;
    use crate::v1::types::{EmojiId, PaginationQuery, PaginationResponse, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[query]
        pub pagination: PaginationQuery<EmojiId>,
    }

    pub struct Response {
        #[json]
        pub emoji: PaginationResponse<EmojiCustom>,
    }
}

/// Emoji search
///
/// Search for custom emoji.
#[endpoint(
    get,
    path = "/emoji/search",
    tags = ["emoji"],
    scopes = [Full],
    response(OK, body = PaginationResponse<EmojiCustom>, description = "success"),
)]
pub mod emoji_search {
    use crate::v1::types::emoji::{EmojiCustom, EmojiSearchQuery};
    use crate::v1::types::{EmojiId, PaginationQuery, PaginationResponse};

    pub struct Request {
        #[query]
        pub search: EmojiSearchQuery,

        #[query]
        pub pagination: PaginationQuery<EmojiId>,
    }

    pub struct Response {
        #[json]
        pub emoji: PaginationResponse<EmojiCustom>,
    }
}

/// Emoji lookup
///
/// Get info about an emoji.
#[endpoint(
    get,
    path = "/emoji/{emoji_id}",
    tags = ["emoji"],
    scopes = [Full],
    response(OK, body = EmojiCustom, description = "success"),
)]
pub mod emoji_lookup {
    use crate::v1::types::emoji::EmojiCustom;
    use crate::v1::types::EmojiId;

    pub struct Request {
        #[path]
        pub emoji_id: EmojiId,
    }

    pub struct Response {
        #[json]
        pub emoji: EmojiCustom,
    }
}
