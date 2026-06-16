use lamprey_macros::endpoint;

/// Channel mirror
///
/// Get incremental sync events for a channel since a given sequence number.
/// Use this to catch up when reconnecting or after being offline.
#[endpoint(
    get,
    path = "/channel/{channel_id}/mirror",
    tags = ["mirror"],
    scopes = [Full],
    permissions = [ChannelView],
    response(OK, body = ChannelSync, description = "channel sync success"),
)]
pub mod channel_mirror {
    use crate::v1::types::{ChannelId, ChannelSeq, ChannelSync, MessageId, PaginationQuery};
    use utoipa::IntoParams;

    #[derive(Debug, IntoParams, serde::Deserialize, serde::Serialize)]
    pub struct SinceQuery {
        /// the sequence number to sync from (exclusive). use 0 to get all events.
        pub since: ChannelSeq,
    }

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[query]
        pub since: SinceQuery,

        #[query]
        pub pagination: PaginationQuery<MessageId>,
    }

    pub struct Response {
        #[json]
        pub sync: ChannelSync,
    }
}

// TODO: room mirror
// TODO: user mirror?
