use lamprey_macros::endpoint;

/// Room audit logs
#[endpoint(
    get,
    path = "/room/{room_id}/audit-logs",
    tags = ["room"],
    scopes = [Rooms],
    permissions = [AuditLogView],
    response(OK, body = AuditLogPaginationResponse, description = "fetch audit logs success"),
)]
pub mod room_audit_logs {
    use crate::v1::types::{
        AuditLogEntryId, AuditLogFilter, AuditLogPaginationResponse, PaginationQuery, RoomId,
    };

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[query]
        pub pagination: PaginationQuery<AuditLogEntryId>,

        #[query]
        pub filter: AuditLogFilter,
    }

    pub struct Response {
        #[json]
        pub logs: AuditLogPaginationResponse,
    }
}

/// User audit logs
#[endpoint(
    get,
    path = "/user/{user_id}/audit-logs",
    tags = ["user"],
    response(OK, body = AuditLogPaginationResponse, description = "success"),
)]
pub mod user_audit_logs {
    use crate::v1::types::{
        AuditLogEntryId, AuditLogFilter, AuditLogPaginationResponse, PaginationQuery,
        misc::UserIdReq,
    };

    pub struct Request {
        #[path]
        pub user_id: UserIdReq,

        #[query]
        pub pagination: PaginationQuery<AuditLogEntryId>,

        #[query]
        pub filter: AuditLogFilter,
    }

    pub struct Response {
        #[json]
        pub logs: AuditLogPaginationResponse,
    }
}
