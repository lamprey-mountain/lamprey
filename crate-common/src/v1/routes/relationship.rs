use lamprey_macros::endpoint;

/// Friend list
///
/// List (mutual) friends.
#[endpoint(
    get,
    path = "/user/@self/friend",
    tags = ["relationship"],
    scopes = [Full],
    response(OK, body = PaginationResponse<RelationshipWithUserId>, description = "success"),
)]
pub mod friend_list {
    use crate::v1::types::{PaginationQuery, PaginationResponse, RelationshipWithUserId, UserId};

    pub struct Request {
        #[query]
        pub pagination: PaginationQuery<UserId>,
    }

    pub struct Response {
        #[json]
        pub friends: PaginationResponse<RelationshipWithUserId>,
    }
}

/// Friend list pending
///
/// List pending friend requests (both incoming and outgoing).
#[endpoint(
    get,
    path = "/user/@self/friend/pending",
    tags = ["relationship"],
    scopes = [Full],
    response(OK, body = PaginationResponse<RelationshipWithUserId>, description = "success"),
)]
pub mod friend_list_pending {
    use crate::v1::types::{PaginationQuery, PaginationResponse, RelationshipWithUserId, UserId};

    pub struct Request {
        #[query]
        pub pagination: PaginationQuery<UserId>,
    }

    pub struct Response {
        #[json]
        pub pending: PaginationResponse<RelationshipWithUserId>,
    }
}

/// Friend add
///
/// Send or accept a friend request.
#[endpoint(
    put,
    path = "/user/@self/friend/{target_id}",
    tags = ["relationship"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod friend_add {
    use crate::v1::types::UserId;

    pub struct Request {
        #[path]
        pub target_id: UserId,
    }

    pub struct Response {}
}

/// Friend remove
///
/// Remove a friend.
#[endpoint(
    delete,
    path = "/user/@self/friend/{target_id}",
    tags = ["relationship"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod friend_remove {
    use crate::v1::types::UserId;

    pub struct Request {
        #[path]
        pub target_id: UserId,
    }

    pub struct Response {}
}

/// Block list
///
/// List blocked users.
#[endpoint(
    get,
    path = "/user/@self/block",
    tags = ["relationship"],
    scopes = [Full],
    response(OK, body = PaginationResponse<RelationshipWithUserId>, description = "success"),
)]
pub mod block_list {
    use crate::v1::types::{PaginationQuery, PaginationResponse, RelationshipWithUserId, UserId};

    pub struct Request {
        #[query]
        pub pagination: PaginationQuery<UserId>,
    }

    pub struct Response {
        #[json]
        pub blocks: PaginationResponse<RelationshipWithUserId>,
    }
}

/// Block add
///
/// Block a user.
#[endpoint(
    put,
    path = "/user/@self/block/{target_id}",
    tags = ["relationship"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod block_add {
    use crate::v1::types::UserId;

    pub struct Request {
        #[path]
        pub target_id: UserId,
    }

    pub struct Response {}
}

/// Block remove
///
/// Unblock a user.
#[endpoint(
    delete,
    path = "/user/@self/block/{target_id}",
    tags = ["relationship"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod block_remove {
    use crate::v1::types::UserId;

    pub struct Request {
        #[path]
        pub target_id: UserId,
    }

    pub struct Response {}
}

/// Ignore add
///
/// Ignore a user's messages.
#[endpoint(
    put,
    path = "/user/@self/ignore/{target_id}",
    tags = ["relationship"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod ignore_add {
    use crate::v1::types::UserId;

    pub struct Request {
        #[path]
        pub target_id: UserId,
    }

    pub struct Response {}
}

/// Ignore remove
///
/// Stop ignoring a user.
#[endpoint(
    delete,
    path = "/user/@self/ignore/{target_id}",
    tags = ["relationship"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod ignore_remove {
    use crate::v1::types::UserId;

    pub struct Request {
        #[path]
        pub target_id: UserId,
    }

    pub struct Response {}
}
