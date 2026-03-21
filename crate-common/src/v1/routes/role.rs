use lamprey_macros::endpoint;

/// Role create
#[endpoint(
    post,
    path = "/room/{room_id}/role",
    tags = ["role"],
    scopes = [Full],
    permissions = [RoleManage],
    response(CREATED, body = Role, description = "success"),
)]
pub mod role_create {
    use crate::v1::types::{Role, RoleCreate, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[json]
        pub role: RoleCreate,

        #[header]
        pub idempotency_key: Option<String>,
    }

    pub struct Response {
        #[json]
        pub role: Role,
    }
}

/// Role update
#[endpoint(
    patch,
    path = "/room/{room_id}/role/{role_id}",
    tags = ["role"],
    scopes = [Full],
    permissions = [RoleManage],
    response(OK, body = Role, description = "success"),
    response(NOT_MODIFIED, description = "success"),
)]
pub mod role_update {
    use crate::v1::types::{Role, RoleId, RolePatch, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[path]
        pub role_id: RoleId,

        #[json]
        pub patch: RolePatch,
    }

    pub struct Response {
        #[json]
        pub role: Role,
    }
}

/// Role delete
#[endpoint(
    delete,
    path = "/room/{room_id}/role/{role_id}",
    tags = ["role"],
    scopes = [Full],
    permissions = [RoleManage],
    response(NO_CONTENT, description = "success"),
)]
pub mod role_delete {
    use crate::v1::types::{RoleId, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[path]
        pub role_id: RoleId,

        #[query]
        pub fallback_role_id: Option<RoleId>,
    }

    pub struct Response {}
}

/// Role list
#[endpoint(
    get,
    path = "/room/{room_id}/role",
    tags = ["role"],
    scopes = [Full],
    response(OK, body = PaginationResponse<Role>, description = "success"),
)]
pub mod role_list {
    use crate::v1::types::{PaginationQuery, PaginationResponse, Role, RoleId, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[query]
        pub pagination: PaginationQuery<RoleId>,
    }

    pub struct Response {
        #[json]
        pub roles: PaginationResponse<Role>,
    }
}

/// Role get
#[endpoint(
    get,
    path = "/room/{room_id}/role/{role_id}",
    tags = ["role"],
    scopes = [Full],
    response(OK, body = Role, description = "success"),
)]
pub mod role_get {
    use crate::v1::types::{Role, RoleId, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[path]
        pub role_id: RoleId,
    }

    pub struct Response {
        #[json]
        pub role: Role,
    }
}

/// Role reorder
#[endpoint(
    patch,
    path = "/room/{room_id}/role",
    tags = ["role"],
    scopes = [Full],
    permissions = [RoleManage],
    response(NO_CONTENT, description = "success"),
)]
pub mod role_reorder {
    use crate::v1::types::{RoleReorder, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[json]
        pub reorder: RoleReorder,
    }

    pub struct Response {}
}

/// Role member bulk patch
#[endpoint(
    patch,
    path = "/room/{room_id}/role/{role_id}/member",
    tags = ["role"],
    scopes = [Full],
    permissions = [RoleManage],
    response(NO_CONTENT, description = "success"),
)]
pub mod role_member_bulk_patch {
    use crate::v1::types::{RoleId, RoleMemberBulkPatch, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[path]
        pub role_id: RoleId,

        #[json]
        pub patch: RoleMemberBulkPatch,
    }

    pub struct Response {}
}

/// Role member list
#[endpoint(
    get,
    path = "/room/{room_id}/role/{role_id}/member",
    tags = ["role"],
    scopes = [Full],
    response(OK, body = PaginationResponse<RoomMember>, description = "success"),
)]
pub mod role_member_list {
    use crate::v1::types::{
        PaginationQuery, PaginationResponse, RoleId, RoomId, RoomMember, UserId,
    };

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[path]
        pub role_id: RoleId,

        #[query]
        pub pagination: PaginationQuery<UserId>,
    }

    pub struct Response {
        #[json]
        pub members: PaginationResponse<RoomMember>,
    }
}

/// Role member add
#[endpoint(
    put,
    path = "/room/{room_id}/role/{role_id}/member/{user_id}",
    tags = ["role"],
    scopes = [Full],
    permissions = [RoleApply],
    response(OK, body = RoomMember, description = "success"),
)]
pub mod role_member_add {
    use crate::v1::types::{RoleId, RoomId, RoomMember, UserId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[path]
        pub role_id: RoleId,

        #[path]
        pub user_id: UserId,
    }

    pub struct Response {
        #[json]
        pub member: RoomMember,
    }
}

/// Role member remove
#[endpoint(
    delete,
    path = "/room/{room_id}/role/{role_id}/member/{user_id}",
    tags = ["role"],
    scopes = [Full],
    permissions = [RoleApply],
    response(NO_CONTENT, description = "success"),
)]
pub mod role_member_remove {
    use crate::v1::types::{RoleId, RoomId, UserId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[path]
        pub role_id: RoleId,

        #[path]
        pub user_id: UserId,
    }

    pub struct Response {}
}
