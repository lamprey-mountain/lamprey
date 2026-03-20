use lamprey_macros::endpoint;

/// Invite delete
#[endpoint(
    delete,
    path = "/invite/{invite_code}",
    tags = ["invite"],
    scopes = [Full],
    permissions_optional = [InviteManage],
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
    response(CREATED, body = Invite, description = "success"),
)]
pub mod invite_create {
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
pub mod invite_list_room {
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
pub mod invite_list_channel {
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

/// Invite update
#[endpoint(
    patch,
    path = "/invite/{invite_code}",
    tags = ["invite"],
    scopes = [Full],
    permissions_optional = [InviteManage],
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
    path = "/invite/{invite_code}/use",
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
