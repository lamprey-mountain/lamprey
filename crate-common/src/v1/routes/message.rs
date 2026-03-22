use lamprey_macros::endpoint;

/// Message create
///
/// Send a message to a channel
#[endpoint(
    post,
    path = "/channel/{channel_id}/message",
    tags = ["message"],
    scopes = [Full],
    permissions = [MessageCreate],
    permissions_optional = [MessageAttachments, MessageEmbeds, IntegrationsBridge],
    response(CREATED, body = Message, description = "Create message success"),
)]
pub mod message_create {
    use crate::v1::types::{ChannelId, Message, MessageCreate};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[json]
        pub message: MessageCreate,

        #[header]
        pub idempotency_key: Option<String>,

        #[header(rename = "x-timestamp")]
        pub timestamp: Option<i64>,
    }

    pub struct Response {
        #[json]
        pub message: Message,
    }
}

/// Message context
///
/// More efficient than calling List messages twice
#[endpoint(
    get,
    path = "/channel/{channel_id}/context/{message_id}",
    tags = ["message"],
    scopes = [Full],
    response(OK, body = ContextResponse, description = "List thread messages success"),
)]
pub mod message_context {
    use crate::v1::types::{ChannelId, ContextQuery, ContextResponse, MessageId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub message_id: MessageId,

        #[query]
        pub context: ContextQuery,
    }

    pub struct Response {
        #[json]
        pub context: ContextResponse,
    }
}

/// Messages list
///
/// Paginate messages in a thread
#[endpoint(
    get,
    path = "/channel/{channel_id}/message",
    tags = ["message"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = PaginationResponse<Message>, description = "List thread messages success"),
)]
pub mod message_list {
    use crate::v1::types::{ChannelId, Message, MessageId, PaginationQuery, PaginationResponse};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[query]
        pub pagination: PaginationQuery<MessageId>,
    }

    pub struct Response {
        #[json]
        pub messages: PaginationResponse<Message>,
    }
}

/// Message get
#[endpoint(
    get,
    path = "/channel/{channel_id}/message/{message_id}",
    tags = ["message"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = Message, description = "Get message success"),
)]
pub mod message_get {
    use crate::v1::types::{ChannelId, Message, MessageId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub message_id: MessageId,
    }

    pub struct Response {
        #[json]
        pub message: Message,
    }
}

/// Message edit
#[endpoint(
    patch,
    path = "/channel/{channel_id}/message/{message_id}",
    tags = ["message"],
    scopes = [Full],
    response(OK, body = Message, description = "edit message success"),
    response(NOT_MODIFIED, description = "no change"),
)]
pub mod message_edit {
    use crate::v1::types::{ChannelId, Message, MessageId};
    use crate::v2::types::message::MessagePatch;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub message_id: MessageId,

        #[json]
        pub patch: MessagePatch,

        #[header(rename = "X-Timestamp")]
        pub timestamp: Option<i64>,
    }

    pub struct Response {
        #[json]
        pub message: Message,
    }
}

/// Message delete
///
/// Note that this endpoint allows deleting your own messages
#[endpoint(
    delete,
    path = "/channel/{channel_id}/message/{message_id}",
    tags = ["message"],
    scopes = [Full],
    permissions_optional = [MessageDelete],
    response(NO_CONTENT, description = "delete message success"),
)]
pub mod message_delete {
    use crate::v1::types::{ChannelId, MessageId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub message_id: MessageId,
    }

    pub struct Response {}
}

/// Message version list
#[endpoint(
    get,
    path = "/channel/{channel_id}/message/{message_id}/version",
    tags = ["message"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = PaginationResponse<Message>, description = "success"),
)]
pub mod message_version_list {
    use crate::v1::types::{
        ChannelId, Message, MessageId, MessageVerId, PaginationQuery, PaginationResponse,
    };

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub message_id: MessageId,

        #[query]
        pub pagination: PaginationQuery<MessageVerId>,
    }

    pub struct Response {
        #[json]
        pub versions: PaginationResponse<Message>,
    }
}

/// Message version get
#[endpoint(
    get,
    path = "/channel/{channel_id}/message/{message_id}/version/{version_id}",
    tags = ["message"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = Message, description = "success"),
)]
pub mod message_version_get {
    use crate::v1::types::{ChannelId, Message, MessageId, MessageVerId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub message_id: MessageId,

        #[path]
        pub version_id: MessageVerId,
    }

    pub struct Response {
        #[json]
        pub message: Message,
    }
}

/// Message version delete
#[endpoint(
    delete,
    path = "/channel/{channel_id}/message/{message_id}/version/{version_id}",
    tags = ["message"],
    scopes = [Full],
    permissions_optional = [MessageDelete],
    response(NO_CONTENT, description = "delete message version success"),
)]
pub mod message_version_delete {
    use crate::v1::types::{ChannelId, MessageId, MessageVerId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub message_id: MessageId,

        #[path]
        pub version_id: MessageVerId,
    }

    pub struct Response {}
}

/// Message moderate
///
/// Bulk remove, restore, or delete messages
#[endpoint(
    patch,
    path = "/channel/{channel_id}/message",
    tags = ["message"],
    scopes = [Full],
    permissions_optional = [MessageDelete, MessageRemove],
    response(OK, description = "success"),
)]
pub mod message_moderate {
    use crate::v1::types::{ChannelId, MessageModerate};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[json]
        pub moderate: MessageModerate,
    }

    pub struct Response {}
}

/// Message migrate
#[endpoint(
    post,
    path = "/channel/{channel_id}/message/migrate",
    tags = ["message"],
    scopes = [Full],
    permissions = [MessageDelete],
    response(OK, description = "success"),
)]
pub mod message_migrate {
    use crate::v1::types::{ChannelId, MessageMigrate};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[json]
        pub migrate: MessageMigrate,
    }

    pub struct Response {}
}

/// Message pin
#[endpoint(
    put,
    path = "/channel/{channel_id}/message/{message_id}/pin",
    tags = ["message"],
    scopes = [Full],
    permissions = [MessagePin],
    response(NO_CONTENT, description = "success"),
)]
pub mod message_pin {
    use crate::v1::types::{ChannelId, MessageId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub message_id: MessageId,
    }

    pub struct Response {}
}

/// Message unpin
#[endpoint(
    delete,
    path = "/channel/{channel_id}/message/{message_id}/pin",
    tags = ["message"],
    scopes = [Full],
    permissions = [MessagePin],
    response(NO_CONTENT, description = "success"),
)]
pub mod message_unpin {
    use crate::v1::types::{ChannelId, MessageId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub message_id: MessageId,
    }

    pub struct Response {}
}

/// Message pins list
#[endpoint(
    get,
    path = "/channel/{channel_id}/pin",
    tags = ["message"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = PaginationResponse<MessagePin>, description = "success"),
)]
pub mod message_pins_list {
    use crate::v1::types::{ChannelId, MessageId, MessagePin, PaginationQuery, PaginationResponse};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[query]
        pub pagination: PaginationQuery<MessageId>,
    }

    pub struct Response {
        #[json]
        pub pins: PaginationResponse<MessagePin>,
    }
}

/// Message pins reorder
#[endpoint(
    patch,
    path = "/channel/{channel_id}/pin",
    tags = ["message"],
    scopes = [Full],
    permissions = [MessagePin],
    response(NO_CONTENT, description = "success"),
)]
pub mod message_pins_reorder {
    use crate::v1::types::{ChannelId, PinsReorder};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[json]
        pub reorder: PinsReorder,
    }

    pub struct Response {}
}

/// Message replies list
#[endpoint(
    get,
    path = "/channel/{channel_id}/message/{message_id}/replies",
    tags = ["message"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = PaginationResponse<Message>, description = "success"),
)]
pub mod message_replies_list {
    use crate::v1::types::{
        ChannelId, Message, MessageId, PaginationQuery, PaginationResponse, RepliesQuery,
    };

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub message_id: MessageId,

        #[query]
        pub replies: RepliesQuery,

        #[query]
        pub pagination: PaginationQuery<MessageId>,
    }

    pub struct Response {
        #[json]
        pub replies: PaginationResponse<Message>,
    }
}

/// Message list deleted
///
/// Paginate deleted messages in a thread
#[endpoint(
    get,
    path = "/channel/{channel_id}/message/deleted",
    tags = ["message"],
    scopes = [Full],
    permissions = [MessageDelete],
    response(OK, body = PaginationResponse<Message>, description = "success"),
)]
pub mod message_list_deleted {
    use crate::v1::types::{ChannelId, Message, MessageId, PaginationQuery, PaginationResponse};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[query]
        pub pagination: PaginationQuery<MessageId>,
    }

    pub struct Response {
        #[json]
        pub messages: PaginationResponse<Message>,
    }
}

/// Message list removed
///
/// Paginate removed messages in a thread
#[endpoint(
    get,
    path = "/channel/{channel_id}/message/removed",
    tags = ["message"],
    scopes = [Full],
    permissions = [MessageRemove],
    response(OK, body = PaginationResponse<Message>, description = "success"),
)]
pub mod message_list_removed {
    use crate::v1::types::{ChannelId, Message, MessageId, PaginationQuery, PaginationResponse};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[query]
        pub pagination: PaginationQuery<MessageId>,
    }

    pub struct Response {
        #[json]
        pub messages: PaginationResponse<Message>,
    }
}

/// Message list atom/rss (TODO)
///
/// Get an atom or rss feed of messages for this channel
#[endpoint(
    get,
    path = "/channel/{channel_id}/message.atom",
    tags = ["message"],
    scopes = [Full],
)]
pub mod message_list_atom {
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,
    }

    pub struct Response {}
}

/// Nudge (TODO)
///
/// Nudge a user. Can only be used in dms or gdms. Can only be called once every 5 minutes per user.
#[endpoint(
    post,
    path = "/channel/{channel_id}/nudge",
    tags = ["message"],
    scopes = [Full],
)]
pub mod message_nudge {
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,
    }

    pub struct Response {}
}
