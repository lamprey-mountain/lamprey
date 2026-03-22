use lamprey_macros::endpoint;

/// Voice state get
#[endpoint(
    get,
    path = "/voice/{channel_id}/member/{user_id}",
    tags = ["voice"],
    scopes = [Full],
    response(OK, body = VoiceState, description = "ok"),
)]
pub mod voice_state_get {
    use crate::v1::types::misc::UserIdReq;
    use crate::v1::types::voice::VoiceState;
    use crate::v1::types::{ChannelId, UserId};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub user_id: UserIdReq,
    }

    pub struct Response {
        #[json]
        pub state: VoiceState,
    }
}

/// Voice state patch
#[endpoint(
    patch,
    path = "/voice/{channel_id}/member/{user_id}",
    tags = ["voice"],
    scopes = [Full],
    permissions_optional = [VoiceMute, VoiceDeafen, VoiceRequest, VoiceMove],
    response(OK, body = VoiceState, description = "ok"),
)]
pub mod voice_state_patch {
    use crate::v1::types::misc::UserIdReq;
    use crate::v1::types::voice::VoiceState;
    use crate::v1::types::voice::VoiceStatePatch;
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub user_id: UserIdReq,

        #[json]
        pub state: VoiceStatePatch,
    }

    pub struct Response {
        #[json]
        pub state: VoiceState,
    }
}

/// Voice state move
#[endpoint(
    put,
    path = "/voice/{channel_id}/member/{user_id}/move",
    tags = ["voice"],
    scopes = [Full],
    permissions = [VoiceMove],
    response(OK, body = VoiceState, description = "ok"),
)]
pub mod voice_state_move {
    use crate::v1::types::misc::UserIdReq;
    use crate::v1::types::voice::VoiceState;
    use crate::v1::types::voice::VoiceStateMove;
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub user_id: UserIdReq,

        #[json]
        pub move_req: VoiceStateMove,
    }

    pub struct Response {
        #[json]
        pub state: VoiceState,
    }
}

/// Voice state move bulk
#[endpoint(
    put,
    path = "/voice/{channel_id}/move-bulk",
    tags = ["voice"],
    scopes = [Full],
    permissions = [VoiceMove],
    response(NO_CONTENT, description = "ok"),
)]
pub mod voice_state_move_bulk {
    use crate::v1::types::voice::VoiceStateMoveBulk;
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[json]
        pub r#move: VoiceStateMoveBulk,
    }

    pub struct Response {}
}

/// Voice call create
#[endpoint(
    post,
    path = "/voice/{channel_id}/call",
    tags = ["voice"],
    scopes = [Full],
    response(CREATED, body = VoiceState, description = "ok"),
)]
pub mod voice_call_create {
    use crate::v1::types::voice::CallCreate;
    use crate::v1::types::voice::VoiceState;
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[json]
        pub call: CallCreate,
    }

    pub struct Response {
        #[json]
        pub state: VoiceState,
    }
}

/// Voice call delete
#[endpoint(
    delete,
    path = "/voice/{channel_id}/call",
    tags = ["voice"],
    scopes = [Full],
    response(NO_CONTENT, description = "ok"),
)]
pub mod voice_call_delete {
    use crate::v1::types::voice::CallDeleteParams;
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[query]
        pub params: CallDeleteParams,
    }

    pub struct Response {}
}

/// Voice call patch
#[endpoint(
    patch,
    path = "/voice/{channel_id}/call",
    tags = ["voice"],
    scopes = [Full],
    response(OK, body = VoiceState, description = "ok"),
)]
pub mod voice_call_patch {
    use crate::v1::types::voice::CallPatch;
    use crate::v1::types::voice::VoiceState;
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[json]
        pub call: CallPatch,
    }

    pub struct Response {
        #[json]
        pub state: VoiceState,
    }
}

/// Voice ring start
#[endpoint(
    post,
    path = "/voice/{channel_id}/ring",
    tags = ["voice"],
    scopes = [Full],
    response(NO_CONTENT, description = "ok"),
)]
pub mod voice_ring_start {
    use crate::v1::types::voice::RingStart;
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[json]
        pub ring: RingStart,
    }

    pub struct Response {}
}

/// Voice ring stop
#[endpoint(
    delete,
    path = "/voice/{channel_id}/ring",
    tags = ["voice"],
    scopes = [Full],
    response(NO_CONTENT, description = "ok"),
)]
pub mod voice_ring_stop {
    use crate::v1::types::voice::RingStop;
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[json]
        pub ring: RingStop,
    }

    pub struct Response {}
}

/// Voice ring eligibility
#[endpoint(
    get,
    path = "/voice/{channel_id}/ring/eligibility",
    tags = ["voice"],
    scopes = [Full],
    response(OK, body = RingEligibility, description = "ok"),
)]
pub mod voice_ring_eligibility {
    use crate::v1::types::voice::RingEligibility;
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,
    }

    pub struct Response {
        #[json]
        pub eligibility: RingEligibility,
    }
}

/// Voice sfu command
#[endpoint(
    post,
    path = "/voice/sfu",
    tags = ["voice"],
    scopes = [Full],
    response(OK, description = "ok"),
)]
pub mod voice_sfu_command {
    use crate::v1::types::voice::SfuCommand;

    pub struct Request {
        #[json]
        pub command: SfuCommand,
    }

    pub struct Response {}
}

/// Voice state disconnect
#[endpoint(
    delete,
    path = "/voice/{channel_id}/member/{user_id}",
    tags = ["voice"],
    scopes = [Full],
    permissions_optional = [VoiceMove],
    response(NO_CONTENT, description = "ok"),
)]
pub mod voice_state_disconnect {
    use crate::v1::types::misc::UserIdReq;
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[path]
        pub user_id: UserIdReq,
    }

    pub struct Response {}
}

/// Voice state disconnect all
#[endpoint(
    delete,
    path = "/voice/{channel_id}/member",
    tags = ["voice"],
    scopes = [Full],
    permissions_optional = [VoiceMove],
    response(NO_CONTENT, description = "ok"),
)]
pub mod voice_state_disconnect_all {
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,
    }

    pub struct Response {}
}

/// Voice state list
#[endpoint(
    get,
    path = "/voice/{channel_id}/member",
    tags = ["voice"],
    scopes = [Full],
    response(OK, body = PaginationResponse<VoiceState>, description = "ok"),
)]
pub mod voice_state_list {
    use crate::v1::types::voice::VoiceState;
    use crate::v1::types::{ChannelId, PaginationResponse};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,
    }

    pub struct Response {
        #[json]
        pub states: PaginationResponse<VoiceState>,
    }
}

/// Voice call get
#[endpoint(
    get,
    path = "/voice/{channel_id}",
    tags = ["voice"],
    scopes = [Full],
    response(OK, body = Call, description = "ok"),
)]
pub mod voice_call_get {
    use crate::v1::types::voice::Call;
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,
    }

    pub struct Response {
        #[json]
        pub call: Call,
    }
}
