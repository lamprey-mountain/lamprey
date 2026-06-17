use lamprey_macros::endpoint;

/// Flume create
///
/// Create a live-updating message in a channel. Flumes allow real-time
/// content updates until committed.
#[endpoint(
    post,
    path = "/channel/{channel_id}/flume",
    tags = ["flume"],
    scopes = [Full],
    permissions = [MessageCreate],
    permissions_optional = [MessageAttachments, MessageEmbeds],
    response(CREATED, body = Message, description = "Flume created successfully"),
)]
pub mod flume_create {
    use crate::v1::types::ChannelId;
    use crate::v1::types::Message;
    use crate::v1::types::message::flume::FlumeCreate;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[json]
        pub flume: FlumeCreate,

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

/// Flume ping
///
/// Keep a flume alive by resetting its autocommit timer. If no ping is
/// received within the autocommit window, the flume will be autocommitted.
#[endpoint(
    post,
    path = "/channel/{channel_id}/flume/{message_id}/ping",
    tags = ["flume"],
    scopes = [Full],
    permissions = [MessageCreate],
    response(NO_CONTENT, description = "Flume pinged successfully"),
)]
pub mod flume_ping {
    use crate::v1::types::{ChannelId, MessageId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub message_id: MessageId,
    }

    pub struct Response {}
}

/// Flume commit
///
/// Commit the flume content, creating a final message version. After commit,
/// no further updates can be applied to this flume.
#[endpoint(
    put,
    path = "/channel/{channel_id}/flume/{message_id}/commit",
    tags = ["flume"],
    scopes = [Full],
    permissions = [MessageCreate],
    response(OK, body = Message, description = "Flume committed successfully"),
)]
pub mod flume_commit {
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

/// Flume update
///
/// Apply a patch to the flume's components. This can append, replace, update,
/// or delete components. The flume must be in the Live state.
#[endpoint(
    patch,
    path = "/channel/{channel_id}/flume/{message_id}/delta",
    tags = ["flume"],
    scopes = [Full],
    permissions = [MessageCreate],
    response(NO_CONTENT, description = "Delta applied successfully"),
    response(NOT_MODIFIED, description = "Delta did not cause any change"),
)]
pub mod flume_delta {
    use crate::v1::types::{ChannelId, MessageId, flume::FlumeDelta};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub message_id: MessageId,

        #[json]
        pub delta: FlumeDelta,
    }

    pub struct Response {}
}
