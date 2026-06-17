use lamprey_macros::endpoint;

/// Pack create
#[endpoint(
    post,
    path = "/pack",
    tags = ["pack"],
    scopes = [Full],
    permissions_server = [RoomCreate],
    response(CREATED, body = Room, description = "success"),
)]
pub mod pack_create {
    use crate::v1::types::{Room, RoomCreate};

    pub struct Request {
        #[json]
        pub pack: RoomCreate,
    }

    pub struct Response {
        #[json]
        pub pack: Room,
    }
}

/// Pack upgrade
///
/// upgrade a room from `type = Pack` to `type = Default`
#[endpoint(
    post,
    path = "/pack/{pack_id}/upgrade",
    tags = ["pack"],
    scopes = [Full],
    permissions = [Admin],
    permissions_server = [RoomCreate],
    response(OK, body = Room, description = "upgraded"),
)]
pub mod pack_upgrade {
    use crate::v1::types::Room;

    pub struct Request {}

    pub struct Response {
        #[json]
        pub room: Room,
    }
}

/// Pack import
///
/// adds all of the emoji in the pack to this room
#[endpoint(
    post,
    path = "/pack/{pack_id}/import",
    tags = ["pack"],
    scopes = [Full],
    permissions = [EmojiManage],
    response(OK, description = "success"),
)]
pub mod pack_import {
    use crate::v1::types::{RoomId, pack::PackImport};

    pub struct Request {
        #[path]
        pub pack_id: RoomId,

        #[json]
        pub body: PackImport,
    }

    pub struct Response {}
}

/// Pack export
#[endpoint(
    get,
    path = "/pack/{pack_id}/export",
    tags = ["pack"],
    scopes = [Rooms],
    response(OK, body = PackSnapshot, description = "success"),
)]
pub mod pack_export {
    use crate::v1::types::{RoomId, pack::PackSnapshot};

    pub struct Request {
        #[path]
        pub pack_id: RoomId,
    }

    pub struct Response {
        #[json]
        pub pack: PackSnapshot,
    }
}

/// Pack list user
#[endpoint(
    get,
    path = "/user/{user_id}/pack",
    tags = ["pack"],
    scopes = [Rooms],
    response(OK, body = PaginationResponse<Room>, description = "success"),
)]
pub mod pack_list_user {
    use crate::v1::types::{PaginationQuery, PaginationResponse, Room, RoomId, UserId};

    pub struct Request {
        #[path]
        pub user_id: UserId,

        #[query]
        pub pagination: PaginationQuery<RoomId>,
    }

    pub struct Response {
        #[json]
        pub packs: PaginationResponse<Room>,
    }
}

/// Pack install user
#[endpoint(
    get,
    path = "/user/{user_id}/pack/{pack_id}",
    tags = ["pack"],
    scopes = [Rooms],
    response(OK, body = PackInstallation, description = "success"),
)]
pub mod pack_install_user {
    use crate::v1::types::{RoomId, UserId, pack::PackInstallation};

    pub struct Request {
        #[path]
        pub user_id: UserId,

        #[path]
        pub pack_id: RoomId,
    }

    pub struct Response {
        #[json]
        pub installation: PackInstallation,
    }
}

/// Pack list room
#[endpoint(
    get,
    path = "/room/{room_id}/pack",
    tags = ["pack"],
    scopes = [Rooms],
    response(OK, body = PaginationResponse<Room>, description = "success"),
)]
pub mod pack_list_room {
    use crate::v1::types::{PaginationQuery, PaginationResponse, Room, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[query]
        pub pagination: PaginationQuery<RoomId>,
    }

    pub struct Response {
        #[json]
        pub packs: PaginationResponse<Room>,
    }
}

/// Pack install room
#[endpoint(
    get,
    path = "/room/{room_id}/pack/{pack_id}",
    tags = ["pack"],
    scopes = [Rooms],
    permissions = [EmojiManage],
    response(OK, body = PackInstallation, description = "success"),
)]
pub mod pack_install_room {
    use crate::v1::types::{RoomId, pack::PackInstallation};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[path]
        pub pack_id: RoomId,
    }

    pub struct Response {
        #[json]
        pub installation: PackInstallation,
    }
}

/// Pack uninstall room
#[endpoint(
    delete,
    path = "/room/{room_id}/pack/{pack_id}",
    tags = ["pack"],
    scopes = [Rooms],
    permissions = [EmojiManage],
    response(NO_CONTENT, description = "success"),
)]
pub mod pack_uninstall_room {
    use crate::v1::types::RoomId;

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[path]
        pub pack_id: RoomId,
    }

    pub struct Response {}
}

/// Pack uninstall user
#[endpoint(
    delete,
    path = "/user/{user_id}/pack/{pack_id}",
    tags = ["pack"],
    scopes = [Rooms],
    response(NO_CONTENT, description = "success"),
)]
pub mod pack_uninstall_user {
    use crate::v1::types::{RoomId, UserId};

    pub struct Request {
        #[path]
        pub user_id: UserId,

        #[path]
        pub pack_id: RoomId,
    }

    pub struct Response {}
}
