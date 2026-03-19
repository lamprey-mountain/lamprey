use lamprey_macros::endpoint;

/// Dm initialize
///
/// Get or create a direct message thread.
#[endpoint(
    post,
    path = "/user/@self/dm/{target_id}",
    tags = ["dm"],
    scopes = [Full],
    permissions = [DmCreate],
    response(CREATED, body = Channel, description = "new dm created"),
    response(OK, body = Channel, description = "already exists"),
)]
pub mod dm_init {
    use crate::v1::types::{Channel, UserId};

    pub struct Request {
        #[path]
        pub target_id: UserId,
    }

    pub struct Response {
        #[json]
        pub channel: Channel,
    }
}

/// Dm get
///
/// Get a direct message room.
#[endpoint(
    get,
    path = "/user/@self/dm/{target_id}",
    tags = ["dm"],
    scopes = [Full],
    response(OK, body = Channel, description = "success"),
)]
pub mod dm_get {
    use crate::v1::types::{Channel, UserId};

    pub struct Request {
        #[path]
        pub target_id: UserId,
    }

    pub struct Response {
        #[json]
        pub channel: Channel,
    }
}

/// Dm list
///
/// List direct message channels.
#[endpoint(
    get,
    path = "/user/{user_id}/dm",
    tags = ["dm"],
    scopes = [Full],
    response(OK, body = PaginationResponse<Channel>, description = "success"),
)]
pub mod dm_list {
    use crate::v1::types::{Channel, MessageVerId, PaginationQuery, PaginationResponse};
    use crate::v1::types::misc::UserIdReq;

    pub struct Request {
        #[path]
        pub user_id: UserIdReq,

        #[query]
        pub pagination: PaginationQuery<MessageVerId>,
    }

    pub struct Response {
        #[json]
        pub channels: PaginationResponse<Channel>,
    }
}
