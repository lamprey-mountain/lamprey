use lamprey_macros::endpoint;

/// Search messages
#[endpoint(
    post,
    path = "/search/message",
    tags = ["search"],
    response(OK, body = MessageSearch, description = "success"),
)]
pub mod search_messages {
    use crate::v1::types::search::{MessageSearch, MessageSearchRequest};

    pub struct Request {
        #[json]
        pub search: MessageSearchRequest,
    }

    pub struct Response {
        #[json]
        pub search: MessageSearch,
    }
}

/// Search channels
#[endpoint(
    post,
    path = "/search/channels",
    tags = ["search"],
    response(OK, body = PaginationResponse<Channel>, description = "success"),
)]
pub mod search_channels {
    use crate::v1::types::search::ChannelSearchRequest;
    use crate::v1::types::{Channel, ChannelId, PaginationQuery, PaginationResponse};

    pub struct Request {
        #[query]
        pub pagination: PaginationQuery<ChannelId>,

        #[json]
        pub search: ChannelSearchRequest,
    }

    pub struct Response {
        #[json]
        pub channels: PaginationResponse<Channel>,
    }
}

/// Search rooms
#[endpoint(
    post,
    path = "/search/room",
    tags = ["search"],
    response(OK, body = PaginationResponse<Room>, description = "success"),
)]
pub mod search_rooms {
    use crate::v1::types::search::RoomSearchRequest;
    use crate::v1::types::{PaginationQuery, PaginationResponse, Room, RoomId};

    pub struct Request {
        #[query]
        pub pagination: PaginationQuery<RoomId>,

        #[json]
        pub search: RoomSearchRequest,
    }

    pub struct Response {
        #[json]
        pub rooms: PaginationResponse<Room>,
    }
}
