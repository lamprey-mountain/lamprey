use lamprey_macros::endpoint;

/// Channel create room
///
/// Create a channel in a room
#[endpoint(
    post,
    path = "/room/{room_id}/channel",
    tags = ["channel"],
    scopes = [Full],
    permissions_optional = [ChannelManage, ThreadCreatePublic, ThreadCreatePrivate],
    audit_log_events = ["ChannelCreate"],
    response(CREATED, body = Channel, description = "Create thread success"),
)]
pub mod channel_create_room {
    use crate::v1::types::{Channel, ChannelCreate, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[json]
        pub channel: ChannelCreate,

        #[header]
        pub idempotency_key: Option<String>,
    }

    pub struct Response {
        #[json]
        pub channel: Channel,
    }
}

/// Channel create dm
///
/// Create a dm or group dm thread (outside of a room)
#[endpoint(
    post,
    path = "/channel",
    tags = ["channel"],
    scopes = [Full],
    permissions = [DmCreate],
    audit_log_events = ["ChannelCreate"],
    response(CREATED, body = Channel, description = "Create thread success"),
    response(OK, body = Channel, description = "already exists"),
)]
pub mod channel_create_dm {
    use crate::v1::types::{Channel, ChannelCreate};

    pub struct Request {
        #[json]
        pub channel: ChannelCreate,

        #[header]
        pub idempotency_key: Option<String>,
    }

    pub struct Response {
        #[json]
        pub channel: Channel,
    }
}

/// Channel get
#[endpoint(
    get,
    path = "/channel/{channel_id}",
    tags = ["channel"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = Channel, description = "Get thread success"),
)]
pub mod channel_get {
    use crate::v1::types::{Channel, ChannelId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,
    }

    pub struct Response {
        #[json]
        pub channel: Channel,
    }
}

/// Room channel list
#[endpoint(
    get,
    path = "/room/{room_id}/channel",
    tags = ["channel"],
    scopes = [Full],
    response(OK, body = PaginationResponse<Channel>, description = "List room channels success"),
)]
pub mod channel_list {
    use crate::v1::types::{Channel, ChannelId, PaginationQuery, PaginationResponse, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[query]
        pub pagination: PaginationQuery<ChannelId>,
    }

    pub struct Response {
        #[json]
        pub channels: PaginationResponse<Channel>,
    }
}

/// Room channel list removed
///
/// List removed threads in a room. Requires the `ChannelManage` permission.
#[endpoint(
    get,
    path = "/room/{room_id}/channel/removed",
    tags = ["channel"],
    scopes = [Full],
    permissions = [ChannelManage],
    response(OK, body = PaginationResponse<Channel>, description = "List removed room threads success"),
)]
pub mod channel_list_removed {
    use crate::v1::types::channel::ChannelListRemovedQuery;
    use crate::v1::types::{Channel, ChannelId, PaginationQuery, PaginationResponse, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[query]
        pub query: ChannelListRemovedQuery,

        #[query]
        pub pagination: PaginationQuery<ChannelId>,
    }

    pub struct Response {
        #[json]
        pub channels: PaginationResponse<Channel>,
    }
}

/// Room channel reorder
///
/// Reorder the channels in a room. Requires the `ChannelManage` permission.
#[endpoint(
    patch,
    path = "/room/{room_id}/channel",
    tags = ["channel"],
    scopes = [Full],
    permissions = [ChannelManage],
    audit_log_events = ["ChannelReorder"],
    response(NO_CONTENT, description = "Reorder channels success"),
)]
pub mod channel_reorder {
    use crate::v1::types::{ChannelReorder, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[json]
        pub reorder: ChannelReorder,
    }

    pub struct Response {}
}

/// Channel update
#[endpoint(
    patch,
    path = "/channel/{channel_id}",
    tags = ["channel"],
    scopes = [Full],
    permissions_optional = [ChannelEdit, ThreadEdit],
    audit_log_events = ["ChannelUpdate"],
    response(OK, body = Channel, description = "edit message success"),
    response(NOT_MODIFIED, body = Channel, description = "no change"),
)]
pub mod channel_update {
    use crate::v1::types::{Channel, ChannelId, ChannelPatch};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[json]
        pub patch: ChannelPatch,
    }

    pub struct Response {
        #[json]
        pub channel: Channel,
    }
}

/// Channel ack
///
/// Mark a channel as read (or unread).
#[endpoint(
    put,
    path = "/channel/{channel_id}/ack",
    tags = ["channel"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = AckRes, description = "success"),
)]
pub mod channel_ack {
    use crate::v1::types::ack::{AckReq, AckRes};
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[json]
        pub ack: AckReq,
    }

    pub struct Response {
        #[json]
        pub ack: AckRes,
    }
}

/// Channel remove
#[endpoint(
    put,
    path = "/channel/{channel_id}/remove",
    tags = ["channel"],
    scopes = [Full],
    permissions = [ThreadManage],
    permissions_optional = [ChannelManage],
    audit_log_events = ["ChannelUpdate"],
    response(NO_CONTENT, description = "success"),
)]
pub mod channel_remove {
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,
    }

    pub struct Response {}
}

/// Channel restore
#[endpoint(
    delete,
    path = "/channel/{channel_id}/remove",
    tags = ["channel"],
    scopes = [Full],
    permissions = [ThreadManage],
    permissions_optional = [ChannelManage],
    audit_log_events = ["ChannelUpdate"],
    response(NO_CONTENT, description = "success"),
)]
pub mod channel_restore {
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,
    }

    pub struct Response {}
}

/// Channel typing
///
/// Send a typing notification to a thread
#[endpoint(
    post,
    path = "/channel/{channel_id}/typing",
    tags = ["channel"],
    scopes = [Full],
    permissions = [MessageCreate],
    response(NO_CONTENT, description = "success"),
)]
pub mod channel_typing {
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,
    }

    pub struct Response {}
}

/// Channel upgrade
///
/// Convert a group dm thread into a full room. Only the gdm creator can upgrade the thread.
#[endpoint(
    post,
    path = "/channel/{channel_id}/upgrade",
    tags = ["channel"],
    scopes = [Full],
    audit_log_events = ["ChannelUpdate"],
    response(OK, body = Room, description = "success"),
)]
pub mod channel_upgrade {
    use crate::v1::types::{ChannelId, Room};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,
    }

    pub struct Response {
        #[json]
        pub room: Room,
    }
}

/// Channel transfer ownership
#[endpoint(
    post,
    path = "/channel/{channel_id}/transfer-ownership",
    tags = ["channel"],
    scopes = [Full],
    audit_log_events = ["ChannelUpdate"],
    response(OK, body = Channel, description = "success"),
)]
pub mod channel_transfer_ownership {
    use crate::v1::types::{Channel, ChannelId, UserId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[json]
        pub owner_id: UserId,
    }

    pub struct Response {
        #[json]
        pub channel: Channel,
    }
}

/// Ratelimit update
///
/// Immediately creates a slowmode ratelimit
/// Requires either ChannelManage or ThreadManage, or MemberTimeout
#[endpoint(
    put,
    path = "/channel/{channel_id}/ratelimit/{user_id}",
    tags = ["channel"],
    scopes = [Full],
    permissions_optional = [ChannelManage, ThreadManage, MemberTimeout],
    audit_log_events = ["RatelimitUpdate"],
    response(OK, description = "Rate limit updated"),
)]
pub mod channel_ratelimit_update {
    use crate::v1::types::{ChannelId, RatelimitPut, UserId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub user_id: UserId,

        #[json]
        pub ratelimit: RatelimitPut,
    }

    pub struct Response {}
}

/// Ratelimit delete
///
/// Immediately expires a slowmode ratelimit, allowing the target user to send a message again
/// Requires either ChannelManage, ThreadManage, or MemberTimeout
#[endpoint(
    delete,
    path = "/channel/{channel_id}/ratelimit/{user_id}",
    tags = ["channel"],
    scopes = [Full],
    permissions_optional = [ChannelManage, ThreadManage, MemberTimeout],
    audit_log_events = ["RatelimitDelete"],
    response(NO_CONTENT, description = "Rate limit expired"),
)]
pub mod channel_ratelimit_delete {
    use crate::v1::types::{ChannelId, UserId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub user_id: UserId,
    }

    pub struct Response {}
}

/// Ratelimit delete all
///
/// Immediately expires a slowmode ratelimit for all users, allowing all users to send messages again
/// Requires either ChannelManage, ThreadManage, or MemberTimeout
#[endpoint(
    delete,
    path = "/channel/{channel_id}/ratelimit",
    tags = ["channel"],
    scopes = [Full],
    permissions_optional = [ChannelManage, ThreadManage, MemberTimeout],
    audit_log_events = ["RatelimitDeleteAll"],
    response(NO_CONTENT, description = "Rate limit expired"),
)]
pub mod channel_ratelimit_delete_all {
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,
    }

    pub struct Response {}
}
