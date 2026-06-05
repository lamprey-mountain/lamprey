use lamprey_macros::endpoint;

/// Room analytics members count
#[endpoint(
    get,
    path = "/room/{room_id}/analytics/members-count",
    tags = ["room_analytics"],
    scopes = [Full],
    permissions = [AnalyticsView],
    response(OK, body = Vec<AnalyticsMembersCount>, description = "success"),
)]
pub mod room_analytics_members_count {
    use crate::v1::types::room_analytics::{AnalyticsMembersCount, AnalyticsParams};
    use crate::v1::types::RoomId;

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[query]
        pub params: AnalyticsParams,
    }

    pub struct Response {
        #[json]
        pub analytics: Vec<AnalyticsMembersCount>,
    }
}

/// Room analytics members join
#[endpoint(
    get,
    path = "/room/{room_id}/analytics/members-join",
    tags = ["room_analytics"],
    scopes = [Full],
    permissions = [AnalyticsView],
    response(OK, body = Vec<AnalyticsMembersJoin>, description = "success"),
)]
pub mod room_analytics_members_join {
    use crate::v1::types::room_analytics::{AnalyticsMembersJoin, AnalyticsParams};
    use crate::v1::types::RoomId;

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[query]
        pub params: AnalyticsParams,
    }

    pub struct Response {
        #[json]
        pub analytics: Vec<AnalyticsMembersJoin>,
    }
}

/// Room analytics members leave
#[endpoint(
    get,
    path = "/room/{room_id}/analytics/members-leave",
    tags = ["room_analytics"],
    scopes = [Full],
    permissions = [AnalyticsView],
    response(OK, body = Vec<AnalyticsMembersLeave>, description = "success"),
)]
pub mod room_analytics_members_leave {
    use crate::v1::types::room_analytics::{AnalyticsMembersLeave, AnalyticsParams};
    use crate::v1::types::RoomId;

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[query]
        pub params: AnalyticsParams,
    }

    pub struct Response {
        #[json]
        pub analytics: Vec<AnalyticsMembersLeave>,
    }
}

/// Room analytics channels
#[endpoint(
    get,
    path = "/room/{room_id}/analytics/channels",
    tags = ["room_analytics"],
    scopes = [Full],
    permissions = [AnalyticsView],
    response(OK, body = Vec<AnalyticsChannel>, description = "success"),
)]
pub mod room_analytics_channels {
    use crate::v1::types::room_analytics::{
        AnalyticsChannel, AnalyticsChannelParams, AnalyticsParams,
    };
    use crate::v1::types::RoomId;

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[query]
        pub params: AnalyticsParams,

        #[query]
        pub channel_params: AnalyticsChannelParams,
    }

    pub struct Response {
        #[json]
        pub analytics: Vec<AnalyticsChannel>,
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
    response(OK, body = Vec<AnalyticsOverview>, description = "success"),
)]
pub mod room_analytics_overview {
    use crate::v1::types::room_analytics::{AnalyticsOverview, AnalyticsParams};
    use crate::v1::types::RoomId;

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[query]
        pub params: AnalyticsParams,
    }

    pub struct Response {
        #[json]
        pub analytics: Vec<AnalyticsOverview>,
    }
}

/// Room analytics invites
#[endpoint(
    get,
    path = "/room/{room_id}/analytics/invites",
    tags = ["room_analytics"],
    scopes = [Full],
    permissions = [AnalyticsView],
    response(OK, body = AnalyticsInvites, description = "success"),
)]
pub mod room_analytics_invites {
    use crate::v1::types::room_analytics::{AnalyticsInvites, AnalyticsParams};
    use crate::v1::types::RoomId;

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[query]
        pub params: AnalyticsParams,
    }

    pub struct Response {
        #[json]
        pub analytics: AnalyticsInvites,
    }
}
