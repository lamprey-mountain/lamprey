use lamprey_macros::endpoint;

/// Thread member list
#[endpoint(
    get,
    path = "/thread/{thread_id}/member",
    tags = ["thread"],
    response(OK, body = PaginationResponse<ThreadMember>, description = "success"),
)]
pub mod thread_member_list {
    use crate::v1::types::{ChannelId, PaginationQuery, PaginationResponse, ThreadMember, UserId};

    pub struct Request {
        #[path]
        pub thread_id: ChannelId,

        #[query]
        pub pagination: PaginationQuery<UserId>,
    }

    pub struct Response {
        #[json]
        pub members: PaginationResponse<ThreadMember>,
    }
}

/// Thread member get
#[endpoint(
    get,
    path = "/thread/{thread_id}/member/{user_id}",
    tags = ["thread"],
    response(OK, body = ThreadMember, description = "success"),
)]
pub mod thread_member_get {
    use crate::v1::types::misc::UserIdReq;
    use crate::v1::types::{ChannelId, ThreadMember};

    pub struct Request {
        #[path]
        pub thread_id: ChannelId,

        #[path]
        pub user_id: UserIdReq,
    }

    pub struct Response {
        #[json]
        pub member: ThreadMember,
    }
}

/// Thread member add
#[endpoint(
    put,
    path = "/thread/{thread_id}/member/{user_id}",
    tags = ["thread"],
    permissions_optional = [MemberKick],
    audit_log_events = ["ThreadMemberAdd"],
    response(OK, body = ThreadMember, description = "success"),
    response(NOT_MODIFIED, description = "not modified"),
)]
pub mod thread_member_add {
    use crate::v1::types::misc::UserIdReq;
    use crate::v1::types::{ChannelId, ThreadMember, ThreadMemberPut};

    pub struct Request {
        #[path]
        pub thread_id: ChannelId,

        #[path]
        pub user_id: UserIdReq,

        #[json]
        pub member: ThreadMemberPut,
    }

    pub struct Response {
        #[json]
        pub member: ThreadMember,
    }
}

/// Thread member delete
#[endpoint(
    delete,
    path = "/thread/{thread_id}/member/{user_id}",
    tags = ["thread"],
    permissions_optional = [MemberKick],
    audit_log_events = ["ThreadMemberRemove"],
    response(NO_CONTENT, description = "success"),
)]
pub mod thread_member_delete {
    use crate::v1::types::misc::UserIdReq;
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub thread_id: ChannelId,

        #[path]
        pub user_id: UserIdReq,
    }

    pub struct Response {}
}

/// Thread list
#[endpoint(
    get,
    path = "/channel/{channel_id}/thread",
    tags = ["thread"],
    permissions = [ChannelView],
    response(OK, body = PaginationResponse<Channel>, description = "List channel threads success"),
)]
pub mod thread_list {
    use crate::v1::types::{Channel, ChannelId, PaginationQuery, PaginationResponse};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[query]
        pub pagination: PaginationQuery<ChannelId>,
    }

    pub struct Response {
        #[json]
        pub threads: PaginationResponse<Channel>,
    }
}

/// Thread list archived
#[endpoint(
    get,
    path = "/channel/{channel_id}/thread/archived",
    tags = ["thread"],
    permissions = [ChannelView],
    response(OK, body = PaginationResponse<Channel>, description = "List archived threads success"),
)]
pub mod thread_list_archived {
    use crate::v1::types::{Channel, ChannelId, PaginationQuery, PaginationResponse};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[query]
        pub pagination: PaginationQuery<ChannelId>,
    }

    pub struct Response {
        #[json]
        pub threads: PaginationResponse<Channel>,
    }
}

/// Thread list removed
#[endpoint(
    get,
    path = "/channel/{channel_id}/thread/removed",
    tags = ["thread"],
    permissions = [ThreadManage],
    response(OK, body = PaginationResponse<Channel>, description = "List removed threads success"),
)]
pub mod thread_list_removed {
    use crate::v1::types::{Channel, ChannelId, PaginationQuery, PaginationResponse};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[query]
        pub pagination: PaginationQuery<ChannelId>,
    }

    pub struct Response {
        #[json]
        pub threads: PaginationResponse<Channel>,
    }
}

/// Thread create
#[endpoint(
    post,
    path = "/channel/{channel_id}/thread",
    tags = ["thread"],
    permissions_optional = [ThreadCreatePublic, ThreadCreatePrivate],
    response(CREATED, body = Channel, description = "Create thread success"),
)]
pub mod thread_create {
    use crate::v1::types::{Channel, ChannelCreate, ChannelId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[json]
        pub thread: ChannelCreate,
    }

    pub struct Response {
        #[json]
        pub thread: Channel,
    }
}

/// Thread create from message
#[endpoint(
    post,
    path = "/channel/{channel_id}/message/{message_id}/thread",
    tags = ["thread"],
    permissions = [ThreadCreatePublic],
    response(CREATED, body = Channel, description = "Create thread success"),
    response(CONFLICT, description = "A thread for this message already exists"),
)]
pub mod thread_create_from_message {
    use crate::v1::types::{Channel, ChannelCreate, ChannelId, MessageId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub message_id: MessageId,

        #[json]
        pub thread: ChannelCreate,
    }

    pub struct Response {
        #[json]
        pub thread: Channel,
    }
}

/// Thread list room
///
/// List all active threads in a room
#[endpoint(
    get,
    path = "/room/{room_id}/thread",
    tags = ["thread"],
    response(OK, body = ThreadListRoom, description = "List room threads success"),
)]
pub mod thread_list_room {
    use crate::v1::types::thread::ThreadListRoom;
    use crate::v1::types::{ChannelId, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,
    }

    pub struct Response {
        #[json]
        pub threads: ThreadListRoom,
    }
}

/// Thread activity
#[endpoint(
    get,
    path = "/channel/{channel_id}/activity",
    tags = ["thread"],
    permissions = [ChannelView],
    response(OK, body = PaginationResponse<Message>, description = "List activity success"),
)]
pub mod thread_activity {
    use crate::v1::types::{ChannelId, Message, MessageId, PaginationQuery, PaginationResponse};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[query]
        pub pagination: PaginationQuery<MessageId>,
    }

    pub struct Response {
        #[json]
        pub activity: PaginationResponse<Message>,
    }
}

/// Channel member search
///
/// If this is a thread, search thread members. Otherwise, search all room members who can view this thread.
#[endpoint(
    get,
    path = "/channel/{channel_id}/member/search",
    tags = ["thread"],
    permissions = [ChannelView],
    response(OK, body = ChannelMemberSearchResponse, description = "success"),
)]
pub mod channel_member_search {
    use crate::v1::types::{ChannelId, ChannelMemberSearch, ChannelMemberSearchResponse};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[query]
        pub search: ChannelMemberSearch,
    }

    pub struct Response {
        #[json]
        pub results: ChannelMemberSearchResponse,
    }
}

/// Thread list atom/rss (TODO)
///
/// Get an atom or rss feed of threads for this channel
#[endpoint(
    get,
    path = "/channel/{channel_id}/thread.atom",
    tags = ["thread"],
)]
pub mod thread_list_atom {
    use crate::v1::types::{ChannelId, PaginationQuery};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[query]
        pub pagination: PaginationQuery<ChannelId>,
    }

    pub struct Response {}
}
