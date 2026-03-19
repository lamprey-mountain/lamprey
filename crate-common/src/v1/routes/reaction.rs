use lamprey_macros::endpoint;

/// Reaction list
///
/// List message reactions for a specific emoji.
#[endpoint(
    get,
    path = "/channel/{channel_id}/message/{message_id}/reaction/{reaction_key}",
    tags = ["reaction"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = PaginationResponse<ReactionListItem>, description = "success"),
)]
pub mod reaction_list {
    use crate::v1::types::{ChannelId, MessageId, PaginationQuery, PaginationResponse, ReactionKeyParam, ReactionListItem, UserId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub message_id: MessageId,

        #[path]
        pub reaction_key: ReactionKeyParam,

        #[query]
        pub pagination: PaginationQuery<UserId>,
    }

    pub struct Response {
        #[json]
        pub reactions: PaginationResponse<ReactionListItem>,
    }
}

/// Reaction add
///
/// Add a reaction to a message.
#[endpoint(
    put,
    path = "/channel/{channel_id}/message/{message_id}/reaction/{reaction_key}/{user_id}",
    tags = ["reaction"],
    scopes = [Full],
    permissions = [ReactionAdd],
    response(CREATED, description = "new reaction created"),
    response(OK, description = "already exists"),
)]
pub mod reaction_add {
    use crate::v1::types::{ChannelId, MessageId, ReactionKeyParam};
    use crate::v1::types::misc::UserIdReq;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub message_id: MessageId,

        #[path]
        pub reaction_key: ReactionKeyParam,

        #[path]
        pub user_id: UserIdReq,
    }

    pub struct Response;
}

/// Reaction remove
///
/// Remove a user's reaction from a message.
#[endpoint(
    delete,
    path = "/channel/{channel_id}/message/{message_id}/reaction/{reaction_key}/{user_id}",
    tags = ["reaction"],
    scopes = [Full],
    permissions_optional = [ReactionPurge],
    response(NO_CONTENT, description = "success"),
)]
pub mod reaction_remove {
    use crate::v1::types::{ChannelId, MessageId, ReactionKeyParam};
    use crate::v1::types::misc::UserIdReq;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub message_id: MessageId,

        #[path]
        pub reaction_key: ReactionKeyParam,

        #[path]
        pub user_id: UserIdReq,
    }

    pub struct Response;
}

/// Reaction remove all
///
/// Remove all reactions from a message.
#[endpoint(
    delete,
    path = "/channel/{channel_id}/message/{message_id}/reaction",
    tags = ["reaction"],
    scopes = [Full],
    permissions = [ReactionPurge],
    response(NO_CONTENT, description = "success"),
)]
pub mod reaction_remove_all {
    use crate::v1::types::{ChannelId, MessageId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub message_id: MessageId,
    }

    pub struct Response;
}

/// Reaction remove emoji
///
/// Remove all reactions of a specific emoji from a message.
#[endpoint(
    delete,
    path = "/channel/{channel_id}/message/{message_id}/reaction/{reaction_key}",
    tags = ["reaction"],
    scopes = [Full],
    permissions = [ReactionPurge],
    response(NO_CONTENT, description = "success"),
)]
pub mod reaction_remove_emoji {
    use crate::v1::types::{ChannelId, MessageId, ReactionKeyParam};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub message_id: MessageId,

        #[path]
        pub reaction_key: ReactionKeyParam,
    }

    pub struct Response;
}
