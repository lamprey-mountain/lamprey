use lamprey_macros::endpoint;

/// Room analytics members count
#[endpoint(
    get,
    path = "/room/{room_id}/analytics/members-count",
    tags = ["room_analytics"],
    scopes = [Full],
    permissions = [AnalyticsView],
    response(OK, body = Vec<RoomAnalyticsMembersCount>, description = "success"),
)]
pub mod room_analytics_members_count {
    use crate::v1::types::room_analytics::{RoomAnalyticsMembersCount, RoomAnalyticsParams};
    use crate::v1::types::RoomId;

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[query]
        pub params: RoomAnalyticsParams,
    }

    pub struct Response {
        #[json]
        pub analytics: Vec<RoomAnalyticsMembersCount>,
    }
}

/// Room analytics members join
#[endpoint(
    get,
    path = "/room/{room_id}/analytics/members-join",
    tags = ["room_analytics"],
    scopes = [Full],
    permissions = [AnalyticsView],
    response(OK, body = Vec<RoomAnalyticsMembersJoin>, description = "success"),
)]
pub mod room_analytics_members_join {
    use crate::v1::types::room_analytics::{RoomAnalyticsMembersJoin, RoomAnalyticsParams};
    use crate::v1::types::RoomId;

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[query]
        pub params: RoomAnalyticsParams,
    }

    pub struct Response {
        #[json]
        pub analytics: Vec<RoomAnalyticsMembersJoin>,
    }
}

/// Room analytics members leave
#[endpoint(
    get,
    path = "/room/{room_id}/analytics/members-leave",
    tags = ["room_analytics"],
    scopes = [Full],
    permissions = [AnalyticsView],
    response(OK, body = Vec<RoomAnalyticsMembersLeave>, description = "success"),
)]
pub mod room_analytics_members_leave {
    use crate::v1::types::room_analytics::{RoomAnalyticsMembersLeave, RoomAnalyticsParams};
    use crate::v1::types::RoomId;

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[query]
        pub params: RoomAnalyticsParams,
    }

    pub struct Response {
        #[json]
        pub analytics: Vec<RoomAnalyticsMembersLeave>,
    }
}

/// Room analytics channels
#[endpoint(
    get,
    path = "/room/{room_id}/analytics/channels",
    tags = ["room_analytics"],
    scopes = [Full],
    permissions = [AnalyticsView],
    response(OK, body = Vec<RoomAnalyticsChannel>, description = "success"),
)]
pub mod room_analytics_channels {
    use crate::v1::types::room_analytics::{RoomAnalyticsChannel, RoomAnalyticsChannelParams, RoomAnalyticsParams};
    use crate::v1::types::RoomId;

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[query]
        pub params: RoomAnalyticsParams,

        #[query]
        pub channel_params: RoomAnalyticsChannelParams,
    }

    pub struct Response {
        #[json]
        pub analytics: Vec<RoomAnalyticsChannel>,
    }
}

/// Room analytics overview
///
/// Aggregate all stats from all channels
#[endpoint(
    get,
    path = "/room/{room_id}/analytics/overview",
    tags = ["room_analytics"],
    scopes = [Full],
    permissions = [AnalyticsView],
    response(OK, body = Vec<RoomAnalyticsOverview>, description = "success"),
)]
pub mod room_analytics_overview {
    use crate::v1::types::room_analytics::{RoomAnalyticsOverview, RoomAnalyticsParams};
    use crate::v1::types::RoomId;

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[query]
        pub params: RoomAnalyticsParams,
    }

    pub struct Response {
        #[json]
        pub analytics: Vec<RoomAnalyticsOverview>,
    }
}

/// Room analytics invites
#[endpoint(
    get,
    path = "/room/{room_id}/analytics/invites",
    tags = ["room_analytics"],
    scopes = [Full],
    permissions = [AnalyticsView],
    response(OK, body = RoomAnalyticsInvites, description = "success"),
)]
pub mod room_analytics_invites {
    use crate::v1::types::room_analytics::{RoomAnalyticsInvites, RoomAnalyticsParams};
    use crate::v1::types::RoomId;

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[query]
        pub params: RoomAnalyticsParams,
    }

    pub struct Response {
        #[json]
        pub analytics: RoomAnalyticsInvites,
    }
}
