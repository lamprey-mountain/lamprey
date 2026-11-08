use lamprey_macros::endpoint;

/// Friend list
///
/// List a user's friends.
///
/// Events include the auth user's relationship with each friend, not the target user's.
#[endpoint(
    get,
    path = "/user/{user_id}/friend",
    tags = ["relationship"],
    scopes = [Full],
    response(OK, body = PaginationResponse<RelationshipWithUserId>, description = "success"),
)]
pub mod friend_list {
    use crate::v1::types::{
        PaginationQuery, PaginationResponse, RelationshipWithUserId, UserId, misc::UserIdReq,
    };

    pub struct Request {
        #[path]
        pub user_id: UserIdReq,

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
/// List the auth user's pending friend requests (both incoming and outgoing).
///
/// Only works for the auth user.
#[endpoint(
    get,
    path = "/user/{user_id}/friend/pending",
    tags = ["relationship"],
    scopes = [Full],
    response(OK, body = PaginationResponse<RelationshipWithUserId>, description = "success"),
)]
pub mod friend_list_pending {
    use crate::v1::types::{
        PaginationQuery, PaginationResponse, RelationshipWithUserId, UserId, misc::UserIdReq,
    };

    pub struct Request {
        #[path]
        pub user_id: UserIdReq,

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
    audit_log_events = ["FriendRequest", "FriendAccept"],
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
    audit_log_events = ["FriendDelete"],
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
/// List the auth user's blocked users.
///
/// Only works for the auth user.
#[endpoint(
    get,
    path = "/user/{user_id}/block",
    tags = ["relationship"],
    scopes = [Full],
    response(OK, body = PaginationResponse<RelationshipWithUserId>, description = "success"),
)]
pub mod block_list {
    use crate::v1::types::{
        PaginationQuery, PaginationResponse, RelationshipWithUserId, UserId, misc::UserIdReq,
    };

    pub struct Request {
        #[path]
        pub user_id: UserIdReq,

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
    audit_log_events = ["BlockCreate"],
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
    audit_log_events = ["BlockDelete"],
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

/// Ignore list
///
/// List the auth user's ignored users.
///
/// Only works for the auth user.
#[endpoint(
    get,
    path = "/user/{user_id}/ignore",
    tags = ["relationship"],
    scopes = [Full],
    response(OK, body = PaginationResponse<RelationshipWithUserId>, description = "success"),
)]
pub mod ignore_list {
    use crate::v1::types::{
        PaginationQuery, PaginationResponse, RelationshipWithUserId, UserId, misc::UserIdReq,
    };

    pub struct Request {
        #[path]
        pub user_id: UserIdReq,

        #[query]
        pub pagination: PaginationQuery<UserId>,
    }

    pub struct Response {
        #[json]
        pub ignored: PaginationResponse<RelationshipWithUserId>,
    }
}

/// Ignore add
///
/// Ignore a user's messages.
#[endpoint(
    put,
    path = "/user/@self/ignore/{target_id}",
    tags = ["relationship"],
    scopes = [Full],
    audit_log_events = ["IgnoreAdd"],
    response(NO_CONTENT, description = "success"),
)]
pub mod ignore_add {
    use crate::v1::types::UserId;
    use crate::v1::types::user::Ignore;

    pub struct Request {
        #[path]
        pub target_id: UserId,

        #[json]
        pub ignore: Ignore,
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
    audit_log_events = ["IgnoreRemove"],
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

/// User rooms list
///
/// List rooms a user is in. If you are not the user, lists mutual rooms.
///
/// Includes the auth user's member object for each room.
#[endpoint(
    get,
    path = "/user/{user_id}/room",
    tags = ["relationship"],
    scopes = [Rooms],
    response(OK, body = RoomList, description = "success"),
)]
pub mod user_room_list {
    use crate::v1::types::{PaginationQuery, RoomId, RoomList, misc::UserIdReq};

    pub struct Request {
        #[path]
        pub user_id: UserIdReq,

        #[query]
        pub pagination: PaginationQuery<RoomId>,
    }

    pub struct Response {
        #[json]
        pub room_list: RoomList,
    }
}
