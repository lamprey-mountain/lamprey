use lamprey_macros::endpoint;

/// User get
///
/// Get another user, including your relationship
#[endpoint(
    get,
    path = "/user/{user_id}",
    tags = ["user"],
    scopes = [Identify],
    response(OK, body = UserWithRelationship, description = "success"),
    errors(UnknownUser),
)]
pub mod user_get {
    use crate::v1::types::{misc::UserIdReq, UserWithRelationship};

    pub struct Request {
        /// the user id to fetch
        #[path]
        pub user_id: UserIdReq,
    }

    pub struct Response {
        #[json]
        pub user: UserWithRelationship,
    }
}

/// User update
#[endpoint(
    patch,
    path = "/user/{user_id}",
    tags = ["user"],
    scopes = [Full],
    permissions = [UserManage],
    permissions_optional = [UserProfileSelf],
    response(OK, body = User, description = "success"),
    response(NOT_MODIFIED, description = "not modified"),
)]
pub mod user_update {
    use crate::v1::types::{misc::UserIdReq, User, UserPatch};

    pub struct Request {
        #[path]
        pub user_id: UserIdReq,

        #[json]
        pub patch: UserPatch,
    }

    pub struct Response {
        #[json]
        pub user: User,
    }
}

/// User delete
#[endpoint(
    delete,
    path = "/user/{user_id}",
    tags = ["user"],
    permissions = [UserManage],
    permissions_optional = [UserManageSelf],
    response(NO_CONTENT, description = "success"),
)]
pub mod user_delete {
    use crate::v1::types::misc::UserIdReq;

    pub struct Request {
        #[path]
        pub user_id: UserIdReq,
    }

    pub struct Response {}
}

/// User undelete
///
/// Allows undeleting a user provided they haven't been garbage collected yet
#[endpoint(
    post,
    path = "/user/{user_id}/undelete",
    tags = ["user"],
    permissions = [UserManage],
    response(NO_CONTENT, description = "success"),
)]
pub mod user_undelete {
    use crate::v1::types::misc::UserIdReq;

    pub struct Request {
        #[path]
        pub user_id: UserIdReq,
    }

    pub struct Response {}
}

/// User rooms list
///
/// List rooms a user is in. If you are not the user, lists mutual rooms.
#[endpoint(
    get,
    path = "/user/{user_id}/room",
    tags = ["user"],
    response(OK, body = PaginationResponse<Room>, description = "success"),
)]
pub mod user_room_list {
    use crate::v1::types::{misc::UserIdReq, PaginationResponse, Room, RoomId};

    pub struct Request {
        #[path]
        pub user_id: UserIdReq,

        #[query]
        pub pagination: crate::v1::types::PaginationQuery<RoomId>,
    }

    pub struct Response {
        #[json]
        pub rooms: PaginationResponse<Room>,
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
        misc::UserIdReq, AuditLogEntryId, AuditLogFilter, AuditLogPaginationResponse,
        PaginationQuery,
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

/// Guest create
///
/// Create a guest account, with limited access to the platform.
///
/// - guests can read but not write public rooms, threads, messages, etc
/// - when using an invite, they can act like a standard account in that one specific room/thread
/// - they can be given an invite to a public room to bypass
#[endpoint(
    post,
    path = "/guest",
    tags = ["user"],
    response(CREATED, body = User, description = "guest account created"),
)]
pub mod guest_create {
    use crate::v1::types::{User, UserCreate};

    pub struct Request {
        #[json]
        pub create: UserCreate,
    }

    pub struct Response {
        #[json]
        pub user: User,
    }
}

/// User suspend
#[endpoint(
    post,
    path = "/user/{user_id}/suspend",
    tags = ["user"],
    permissions = [MemberBan],
    response(OK, body = User, description = "success"),
)]
pub mod user_suspend {
    use crate::v1::types::{misc::UserIdReq, SuspendRequest, User};

    pub struct Request {
        #[path]
        pub user_id: UserIdReq,

        #[json]
        pub suspend: SuspendRequest,

        #[header]
        pub reason: Option<String>,
    }

    pub struct Response {
        #[json]
        pub user: User,
    }
}

/// User unsuspend
#[endpoint(
    delete,
    path = "/user/{user_id}/suspend",
    tags = ["user"],
    permissions = [MemberBan],
    response(OK, body = User, description = "success"),
)]
pub mod user_unsuspend {
    use crate::v1::types::{misc::UserIdReq, User};

    pub struct Request {
        #[path]
        pub user_id: UserIdReq,
    }

    pub struct Response {
        #[json]
        pub user: User,
    }
}

/// User presence set
///
/// for puppets
#[endpoint(
    post,
    path = "/user/{user_id}/presence",
    tags = ["user"],
    response(NO_CONTENT, description = "success"),
)]
pub mod user_presence_set {
    use crate::v1::types::{misc::UserIdReq, presence::Presence};

    pub struct Request {
        #[path]
        pub user_id: UserIdReq,

        #[json]
        pub presence: Presence,
    }

    pub struct Response {}
}

/// User list
///
/// Admin only. List all users on this server.
#[endpoint(
    get,
    path = "/user",
    tags = ["user"],
    permissions = [MemberBan],
    response(OK, body = PaginationResponse<User>, description = "success"),
)]
pub mod user_list {
    use crate::v1::types::{PaginationQuery, PaginationResponse, User, UserId, UserListFilter};

    pub struct Request {
        #[query]
        pub pagination: PaginationQuery<UserId>,

        #[query]
        pub filter: Option<UserListFilter>,
    }

    pub struct Response {
        #[json]
        pub users: PaginationResponse<User>,
    }
}

/// Harvest get
#[endpoint(
    get,
    path = "/user/@self/harvest",
    tags = ["user"],
    response(OK, body = Harvest, description = "success"),
    response(NOT_FOUND, description = "no harvest found"),
)]
pub mod harvest_get {
    use crate::v1::types::harvest::Harvest;

    pub struct Request {}

    pub struct Response {
        #[json]
        pub harvest: Harvest,
    }
}

/// Harvest create
#[endpoint(
    post,
    path = "/user/@self/harvest",
    tags = ["user"],
    response(ACCEPTED, description = "harvest has been queued"),
)]
pub mod harvest_create {
    use crate::v1::types::harvest::HarvestCreate;

    pub struct Request {
        #[json]
        pub harvest: HarvestCreate,
    }

    pub struct Response {}
}

/// Harvest download
#[endpoint(
    get,
    path = "/internal/harvest/{harvest_id}/{token}/download",
    tags = ["user"],
    response(OK, description = "success"),
)]
pub mod harvest_download {
    use crate::v1::types::HarvestId;

    pub struct Request {
        #[path]
        pub harvest_id: HarvestId,

        #[path]
        pub token: String,
    }

    pub struct Response {}
}

/// User search
#[endpoint(
    post,
    path = "/user/search",
    tags = ["user"],
    permissions = [Admin],
    response(OK, description = "success"),
)]
pub mod user_search {
    use crate::v1::types::UserSearch;

    pub struct Request {
        #[json]
        pub search: UserSearch,
    }

    pub struct Response {}
}
