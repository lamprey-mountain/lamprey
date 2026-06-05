use lamprey_macros::endpoint;

/// Harvest user get
#[endpoint(
    get,
    path = "/user/{user_id}/harvest",
    tags = ["harvest"],
    response(OK, body = Harvest, description = "success"),
    response(NOT_FOUND, description = "no harvest found"),
)]
pub mod harvest_user_get {
    use crate::v1::types::{harvest::Harvest, misc::UserIdReq};

    pub struct Request {
        #[path]
        pub user_id: UserIdReq,
    }

    pub struct Response {
        #[json]
        pub harvest: Harvest,
    }
}

/// Harvest user create
#[endpoint(
    post,
    path = "/user/{user_id}/harvest",
    tags = ["harvest"],
    audit_log_events = ["HarvestCreate"],
    response(ACCEPTED, description = "harvest has been queued"),
)]
pub mod harvest_user_create {
    use crate::v1::types::{harvest::HarvestCreateUser, misc::UserIdReq};

    pub struct Request {
        #[path]
        pub user_id: UserIdReq,

        #[json]
        pub harvest: HarvestCreateUser,
    }

    pub struct Response {}
}

/// Harvest room get
#[endpoint(
    get,
    path = "/room/{room_id}/harvest",
    tags = ["harvest"],
    permissions = [Admin],
    response(OK, body = Harvest, description = "success"),
    response(NOT_FOUND, description = "no harvest found"),
)]
pub mod harvest_room_get {
    use crate::v1::types::{harvest::Harvest, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,
    }

    pub struct Response {
        #[json]
        pub harvest: Harvest,
    }
}

/// Harvest room create
#[endpoint(
    post,
    path = "/room/{room_id}/harvest",
    tags = ["harvest"],
    permissions = [Admin],
    audit_log_events = ["HarvestCreate"],
    response(ACCEPTED, description = "harvest has been queued"),
)]
pub mod harvest_room_create {
    use crate::v1::types::{harvest::HarvestCreateRoom, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[json]
        pub harvest: HarvestCreateRoom,
    }

    pub struct Response {}
}

/// Harvest get
#[endpoint(
    get,
    path = "/harvest/{harvest_id}",
    tags = ["harvest"],
    permissions_optional = [Admin],
    response(OK, description = "success"),
)]
pub mod harvest_get {
    use crate::v1::types::{harvest::Harvest, HarvestId};

    pub struct Request {
        #[path]
        pub harvest_id: HarvestId,
    }

    pub struct Response {
        #[json]
        pub harvest: Harvest,
    }
}

// TODO: harvest_list
// TODO: harvest_search
