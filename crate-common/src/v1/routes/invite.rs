use lamprey_macros::endpoint;

/// Invite delete
#[endpoint(
    delete,
    path = "/invite/{invite_code}",
    tags = ["invite"],
    scopes = [Full],
    permissions_optional = [InviteManage],
    audit_log_events = ["InviteDelete"],
    response(NO_CONTENT, description = "success"),
)]
pub mod invite_delete {
    use crate::v1::types::InviteCode;

    pub struct Request {
        #[path]
        pub invite_code: InviteCode,
    }

    pub struct Response {}
}

/// Invite resolve
#[endpoint(
    get,
    path = "/invite/{invite_code}",
    tags = ["invite"],
    scopes = [Full],
    response(OK, body = Invite, description = "success"),
    response(OK, body = InviteWithMetadata, description = "success with metadata"),
)]
pub mod invite_resolve {
    use crate::v1::types::{Invite, InviteCode, InviteWithMetadata};

    pub struct Request {
        #[path]
        pub invite_code: InviteCode,
    }

    pub struct Response {
        #[json]
        pub invite: Invite,
    }
}

/// Invite create
#[endpoint(
    post,
    path = "/room/{room_id}/invite",
    tags = ["invite"],
    scopes = [Full],
    permissions_optional = [InviteManage],
    audit_log_events = ["InviteCreate"],
    response(CREATED, body = Invite, description = "success"),
)]
pub mod invite_room_create {
    use crate::v1::types::{Invite, InviteCreate, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[json]
        pub invite: InviteCreate,
    }

    pub struct Response {
        #[json]
        pub invite: Invite,
    }
}

/// Invite list room
#[endpoint(
    get,
    path = "/room/{room_id}/invite",
    tags = ["invite"],
    scopes = [Full],
    permissions_optional = [InviteManage],
    response(OK, body = PaginationResponse<Invite>, description = "success"),
)]
pub mod invite_room_list {
    use crate::v1::types::{Invite, InviteCode, PaginationQuery, PaginationResponse, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[query]
        pub pagination: PaginationQuery<InviteCode>,
    }

    pub struct Response {
        #[json]
        pub invites: PaginationResponse<Invite>,
    }
}

/// Invite list channel
#[endpoint(
    get,
    path = "/channel/{channel_id}/invite",
    tags = ["invite"],
    scopes = [Full],
    permissions_optional = [InviteManage],
    response(OK, body = PaginationResponse<Invite>, description = "success"),
)]
pub mod invite_channel_list {
    use crate::v1::types::{ChannelId, Invite, InviteCode, PaginationQuery, PaginationResponse};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[query]
        pub pagination: PaginationQuery<InviteCode>,
    }

    pub struct Response {
        #[json]
        pub invites: PaginationResponse<Invite>,
    }
}

/// Invite channel create
#[endpoint(
    post,
    path = "/channel/{channel_id}/invite",
    tags = ["invite"],
    scopes = [Full],
    permissions_optional = [InviteManage],
    audit_log_events = ["InviteCreate"],
    response(CREATED, body = Invite, description = "success"),
)]
pub mod invite_channel_create {
    use crate::v1::types::{ChannelId, Invite, InviteCreate};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[json]
        pub invite: InviteCreate,
    }

    pub struct Response {
        #[json]
        pub invite: Invite,
    }
}

/// Invite update
#[endpoint(
    patch,
    path = "/invite/{invite_code}",
    tags = ["invite"],
    scopes = [Full],
    permissions_optional = [InviteManage],
    audit_log_events = ["InviteUpdate"],
    response(OK, body = Invite, description = "success"),
)]
pub mod invite_update {
    use crate::v1::types::{Invite, InviteCode, InvitePatch};

    pub struct Request {
        #[path]
        pub invite_code: InviteCode,

        #[json]
        pub patch: InvitePatch,
    }

    pub struct Response {
        #[json]
        pub invite: Invite,
    }
}

/// Invite use
#[endpoint(
    post,
    path = "/invite/{invite_code}",
    tags = ["invite"],
    scopes = [Full],
    response(OK, description = "success"),
)]
pub mod invite_use {
    use crate::v1::types::InviteCode;

    pub struct Request {
        #[path]
        pub invite_code: InviteCode,
    }

    pub struct Response {}
}

/// Invite server create
#[endpoint(
    post,
    path = "/server/invite",
    tags = ["invite"],
    scopes = [Full],
    permissions_optional = [InviteCreate],
    response(CREATED, body = Invite, description = "success"),
)]
pub mod invite_server_create {
    use crate::v1::types::{Invite, InviteCreate};

    pub struct Request {
        #[json]
        pub invite: InviteCreate,
    }

    pub struct Response {
        #[json]
        pub invite: Invite,
    }
}

/// Invite server list
#[endpoint(
    get,
    path = "/server/invite",
    tags = ["invite"],
    scopes = [Full],
    permissions_optional = [InviteManage],
    response(OK, body = PaginationResponse<Invite>, description = "success"),
)]
pub mod invite_server_list {
    use crate::v1::types::{Invite, InviteCode, PaginationQuery, PaginationResponse};

    pub struct Request {
        #[query]
        pub pagination: PaginationQuery<InviteCode>,
    }

    pub struct Response {
        #[json]
        pub invites: PaginationResponse<Invite>,
    }
}

/// Invite user create
#[endpoint(
    post,
    path = "/user/{user_id}/invite",
    tags = ["invite"],
    scopes = [Full],
    permissions_optional = [InviteCreate],
    response(CREATED, body = Invite, description = "success"),
)]
pub mod invite_user_create {
    use crate::v1::types::misc::UserIdReq;
    use crate::v1::types::{Invite, InviteCreate};

    pub struct Request {
        #[path]
        pub user_id: UserIdReq,

        #[json]
        pub invite: InviteCreate,
    }

    pub struct Response {
        #[json]
        pub invite: Invite,
    }
}

/// Invite user list
#[endpoint(
    get,
    path = "/user/{user_id}/invite",
    tags = ["invite"],
    scopes = [Full],
    response(OK, body = PaginationResponse<Invite>, description = "success"),
)]
pub mod invite_user_list {
    use crate::v1::types::misc::UserIdReq;
    use crate::v1::types::{Invite, InviteCode, PaginationQuery, PaginationResponse};

    pub struct Request {
        #[path]
        pub user_id: UserIdReq,

        #[query]
        pub pagination: PaginationQuery<InviteCode>,
    }

    pub struct Response {
        #[json]
        pub invites: PaginationResponse<Invite>,
    }
}
