use lamprey_macros::endpoint;

/// Room member list
#[endpoint(
    get,
    path = "/room/{room_id}/member",
    tags = ["room_member"],
    scopes = [Full],
    response(OK, body = PaginationResponse<RoomMember>, description = "success"),
)]
pub mod room_member_list {
    use crate::v1::types::{PaginationQuery, PaginationResponse, RoomId, RoomMember, UserId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[query]
        pub pagination: PaginationQuery<UserId>,
    }

    pub struct Response {
        #[json]
        pub members: PaginationResponse<RoomMember>,
    }
}

/// Room member get
#[endpoint(
    get,
    path = "/room/{room_id}/member/{user_id}",
    tags = ["room_member"],
    scopes = [Full],
    response(OK, body = RoomMember, description = "success"),
)]
pub mod room_member_get {
    use crate::v1::types::misc::UserIdReq;
    use crate::v1::types::{RoomId, RoomMember};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[path]
        pub user_id: UserIdReq,
    }

    pub struct Response {
        #[json]
        pub member: RoomMember,
    }
}

/// Room member add
#[endpoint(
    put,
    path = "/room/{room_id}/member/{user_id}",
    tags = ["room_member"],
    scopes = [Full],
    permissions_optional = [IntegrationsBridge, VoiceMute, VoiceDeafen, MemberKick, RoleApply],
    response(OK, body = RoomMember, description = "success"),
    response(NOT_MODIFIED, description = "not modified"),
)]
pub mod room_member_add {
    use crate::v1::types::misc::UserIdReq;
    use crate::v1::types::{RoomId, RoomMember, RoomMemberPut};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[path]
        pub user_id: UserIdReq,

        #[json]
        pub member: RoomMemberPut,
    }

    pub struct Response {
        #[json]
        pub member: RoomMember,
    }
}

/// Room member update
#[endpoint(
    patch,
    path = "/room/{room_id}/member/{user_id}",
    tags = ["room_member"],
    scopes = [Full],
    permissions_optional = [VoiceMute, VoiceDeafen, MemberKick, RoleApply, MemberTimeout, MemberNickname, MemberNicknameManage],
    response(OK, body = RoomMember, description = "success"),
    response(NOT_MODIFIED, description = "not modified"),
)]
pub mod room_member_update {
    use crate::v1::types::misc::UserIdReq;
    use crate::v1::types::{RoomId, RoomMember, RoomMemberPatch};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[path]
        pub user_id: UserIdReq,

        #[json]
        pub patch: RoomMemberPatch,
    }

    pub struct Response {
        #[json]
        pub member: RoomMember,
    }
}

/// Room member delete
#[endpoint(
    delete,
    path = "/room/{room_id}/member/{user_id}",
    tags = ["room_member"],
    scopes = [Full],
    permissions_optional = [MemberKick],
    response(NO_CONTENT, description = "success"),
)]
pub mod room_member_delete {
    use crate::v1::types::misc::UserIdReq;
    use crate::v1::types::{RoomId, RoomMemberOrigin};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[path]
        pub user_id: UserIdReq,

        #[json]
        pub origin: Option<RoomMemberOrigin>,
    }

    pub struct Response {}
}

/// Room member search
#[endpoint(
    get,
    path = "/room/{room_id}/member/search",
    tags = ["room_member"],
    scopes = [Full],
    response(OK, body = RoomMemberSearchResponse, description = "success"),
)]
pub mod room_member_search {
    use crate::v1::types::{RoomId, RoomMemberSearch, RoomMemberSearchResponse};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[query]
        pub search: RoomMemberSearch,
    }

    pub struct Response {
        #[json]
        pub results: RoomMemberSearchResponse,
    }
}

/// Room member search advanced
#[endpoint(
    post,
    path = "/room/{room_id}/member/search/advanced",
    tags = ["room_member"],
    scopes = [Full],
    response(OK, body = RoomMemberSearchResponse, description = "success"),
)]
pub mod room_member_search_advanced {
    use crate::v1::types::{RoomId, RoomMemberSearchAdvanced, RoomMemberSearchResponse};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[json]
        pub search: RoomMemberSearchAdvanced,
    }

    pub struct Response {
        #[json]
        pub results: RoomMemberSearchResponse,
    }
}

/// Room ban create
#[endpoint(
    post,
    path = "/room/{room_id}/ban/{user_id}",
    tags = ["room_member"],
    scopes = [Full],
    permissions = [MemberBan],
    response(NO_CONTENT, description = "success"),
)]
pub mod room_ban_create {
    use crate::v1::types::misc::UserIdReq;
    use crate::v1::types::{RoomBanCreate, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[path]
        pub user_id: UserIdReq,

        #[json]
        pub ban: RoomBanCreate,
    }

    pub struct Response {}
}

/// Room ban list
#[endpoint(
    get,
    path = "/room/{room_id}/ban",
    tags = ["room_member"],
    scopes = [Full],
    permissions = [MemberBan],
    response(OK, body = PaginationResponse<RoomBan>, description = "success"),
)]
pub mod room_ban_list {
    use crate::v1::types::{PaginationQuery, PaginationResponse, RoomBan, RoomId, UserId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[query]
        pub pagination: PaginationQuery<UserId>,
    }

    pub struct Response {
        #[json]
        pub bans: PaginationResponse<RoomBan>,
    }
}

/// Room ban get
#[endpoint(
    get,
    path = "/room/{room_id}/ban/{user_id}",
    tags = ["room_member"],
    scopes = [Full],
    permissions = [MemberBan],
    response(OK, body = RoomBan, description = "success"),
)]
pub mod room_ban_get {
    use crate::v1::types::misc::UserIdReq;
    use crate::v1::types::{RoomBan, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[path]
        pub user_id: UserIdReq,
    }

    pub struct Response {
        #[json]
        pub ban: RoomBan,
    }
}

/// Room ban delete
#[endpoint(
    delete,
    path = "/room/{room_id}/ban/{user_id}",
    tags = ["room_member"],
    scopes = [Full],
    permissions = [MemberBan],
    response(NO_CONTENT, description = "success"),
)]
pub mod room_ban_delete {
    use crate::v1::types::misc::UserIdReq;
    use crate::v1::types::RoomId;

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[path]
        pub user_id: UserIdReq,
    }

    pub struct Response {}
}

/// Room ban bulk create
#[endpoint(
    post,
    path = "/room/{room_id}/ban/bulk",
    tags = ["room_member"],
    scopes = [Full],
    permissions = [MemberBan],
    response(OK, description = "success"),
)]
pub mod room_ban_bulk_create {
    use crate::v1::types::{RoomBanBulkCreate, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[json]
        pub ban: RoomBanBulkCreate,
    }

    pub struct Response {}
}

/// Room prune begin
#[endpoint(
    post,
    path = "/room/{room_id}/prune/begin",
    tags = ["room_member"],
    scopes = [Full],
    permissions = [MemberKick],
    response(OK, body = PruneResponse, description = "success"),
)]
pub mod room_prune_begin {
    use crate::v1::types::{PruneBegin, PruneResponse, RoomId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[json]
        pub prune: PruneBegin,
    }

    pub struct Response {
        #[json]
        pub prune: PruneResponse,
    }
}

/// Room ban search
#[endpoint(
    get,
    path = "/room/{room_id}/ban/search",
    tags = ["room_member"],
    scopes = [Full],
    permissions = [MemberBan],
    response(OK, body = PaginationResponse<RoomBan>, description = "success"),
)]
pub mod room_ban_search {
    use crate::v1::types::{PaginationQuery, PaginationResponse, RoomBan, RoomId, UserId};

    pub struct Request {
        #[path]
        pub room_id: RoomId,

        #[query]
        pub query: String,

        #[query]
        pub pagination: PaginationQuery<UserId>,
    }

    pub struct Response {
        #[json]
        pub bans: PaginationResponse<RoomBan>,
    }
}
