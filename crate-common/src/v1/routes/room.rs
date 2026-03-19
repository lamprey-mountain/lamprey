use lamprey_macros::endpoint;

/// Room create
#[endpoint(
    post,
    path = "/room",
    tags = ["room"],
    scopes = [Full],
    permissions = [RoomCreate],
    response(CREATED, body = Room, description = "success"),
)]
pub mod room_create {
    use crate::v1::types::{Room, RoomCreate};

    pub struct Request {
        #[json]
        pub room: RoomCreate,

        #[header]
        pub idempotency_key: Option<String>,
    }

    pub struct Response {
        #[json]
        pub room: Room,
    }
}

/// Room get
#[endpoint(
    get,
    path = "/room/{room_id}",
    tags = ["room"],
    scopes = [Rooms],
    response(OK, body = Room, description = "Get room success"),
    response(NOT_MODIFIED, description = "Not modified"),
)]
pub mod room_get {
    use crate::v1::types::{Room, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[header]
        pub if_none_match: Option<String>,
    }

    pub struct Response {
        #[json]
        pub room: Room,
    }
}

/// Room list
///
/// Lists all rooms on the server.
#[endpoint(
    get,
    path = "/room",
    tags = ["room"],
    scopes = [Rooms],
    permissions = [RoomManage],
    response(OK, body = PaginationResponse<Room>, description = "Paginate room success"),
)]
pub mod room_list {
    use crate::v1::types::{PaginationQuery, PaginationResponse, Room, RoomId};

    pub struct Request {
        #[query]
        pub pagination: PaginationQuery<RoomId>,
    }

    pub struct Response {
        #[json]
        pub rooms: PaginationResponse<Room>,
    }
}

/// Room search
#[endpoint(
    post,
    path = "/room/search",
    tags = ["room"],
    scopes = [Full],
    permissions = [RoomManage],
    response(OK, description = "success"),
)]
pub mod room_search {
    use crate::v1::types::search::RoomSearchRequest;

    pub struct Request {
        #[json]
        pub search: RoomSearchRequest,
    }

    pub struct Response {}
}

/// Room edit
#[endpoint(
    patch,
    path = "/room/{room_id}",
    tags = ["room"],
    scopes = [Full],
    permissions = [RoomEdit],
    response(OK, body = Room, description = "edit success"),
    response(NOT_MODIFIED, description = "no change"),
)]
pub mod room_edit {
    use crate::v1::types::{Room, RoomId, RoomPatch};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[json]
        pub patch: RoomPatch,
    }

    pub struct Response {
        #[json]
        pub room: Room,
    }
}

/// Room delete
#[endpoint(
    delete,
    path = "/room/{room_id}",
    tags = ["room"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod room_delete {
    use crate::v1::types::RoomId;

    pub struct Request {
        #[path]
        pub room_id: RoomId,
    }

    pub struct Response {}
}

/// Room undelete
#[endpoint(
    post,
    path = "/room/{room_id}/undelete",
    tags = ["room"],
    scopes = [Full],
    permissions = [RoomManage],
    response(NO_CONTENT, description = "success"),
)]
pub mod room_undelete {
    use crate::v1::types::RoomId;

    pub struct Request {
        #[path]
        pub room_id: RoomId,
    }

    pub struct Response {}
}

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

/// Room ack
///
/// Mark all channels in a room as read.
#[endpoint(
    put,
    path = "/room/{room_id}/ack",
    tags = ["room"],
    scopes = [Rooms],
    response(OK, description = "success"),
)]
pub mod room_ack {
    use crate::v1::types::RoomId;

    pub struct Request {
        #[path]
        pub room_id: RoomId,
    }

    pub struct Response {}
}

/// Room transfer ownership
#[endpoint(
    post,
    path = "/room/{room_id}/transfer-ownership",
    tags = ["room"],
    scopes = [Full],
    response(OK, body = Room, description = "success"),
)]
pub mod room_transfer_ownership {
    use crate::v1::types::{Room, RoomId, TransferOwnership};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[json]
        pub transfer: TransferOwnership,
    }

    pub struct Response {
        #[json]
        pub room: Room,
    }
}

/// Room integration list
///
/// list bots in a room
#[endpoint(
    get,
    path = "/room/{room_id}/integration",
    tags = ["room"],
    scopes = [Rooms],
    response(OK, body = PaginationResponse<Integration>, description = "success"),
)]
pub mod room_integration_list {
    use crate::v1::types::application::Integration;
    use crate::v1::types::{ApplicationId, PaginationQuery, PaginationResponse, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[query]
        pub pagination: PaginationQuery<ApplicationId>,
    }

    pub struct Response {
        #[json]
        pub integrations: PaginationResponse<Integration>,
    }
}

/// Room quarantine
#[endpoint(
    post,
    path = "/room/{room_id}/quarantine",
    tags = ["room"],
    scopes = [Full],
    permissions = [RoomManage],
    response(OK, body = Room, description = "success"),
)]
pub mod room_quarantine {
    use crate::v1::types::{Room, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,
    }

    pub struct Response {
        #[json]
        pub room: Room,
    }
}

/// Room unquarantine
#[endpoint(
    delete,
    path = "/room/{room_id}/quarantine",
    tags = ["room"],
    scopes = [Full],
    permissions = [RoomManage],
    response(OK, body = Room, description = "success"),
)]
pub mod room_unquarantine {
    use crate::v1::types::{Room, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,
    }

    pub struct Response {
        #[json]
        pub room: Room,
    }
}

/// Room security set
#[endpoint(
    put,
    path = "/room/{room_id}/security",
    tags = ["room"],
    scopes = [Full],
    response(OK, body = Room, description = "success"),
)]
pub mod room_security_set {
    use crate::v1::types::{Room, RoomId, RoomSecurityUpdate};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[json]
        pub security: RoomSecurityUpdate,
    }

    pub struct Response {
        #[json]
        pub room: Room,
    }
}
