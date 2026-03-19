use lamprey_macros::endpoint;

/// Report create server
///
/// Create and send a report to the server operators
#[endpoint(
    post,
    path = "/server/report",
    tags = ["moderation"],
    scopes = [Full],
    response(OK, body = Report, description = "success"),
)]
pub mod report_create_server {
    use crate::v1::types::moderation::{Report, ReportCreate};

    pub struct Request {
        #[json]
        pub report: ReportCreate,
    }

    pub struct Response {
        #[json]
        pub report: Report,
    }
}

/// Report create room
///
/// Create and send a report to the room admins/moderators
#[endpoint(
    post,
    path = "/room/{room_id}/report",
    tags = ["moderation"],
    scopes = [Full],
    response(OK, body = Report, description = "success"),
)]
pub mod report_create_room {
    use crate::v1::types::moderation::{Report, ReportCreate};
    use crate::v1::types::RoomId;

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[json]
        pub report: ReportCreate,
    }

    pub struct Response {
        #[json]
        pub report: Report,
    }
}
