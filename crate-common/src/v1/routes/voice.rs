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
    use crate::v1::types::ChannelId;

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
    use crate::v1::types::voice::{VoiceState, VoiceStatePatch};
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
    post,
    path = "/voice/{channel_id}/member/{user_id}/move",
    tags = ["voice"],
    scopes = [Full],
    permissions = [VoiceMove],
    response(OK, body = VoiceState, description = "ok"),
)]
pub mod voice_state_move {
    use crate::v1::types::misc::UserIdReq;
    use crate::v1::types::voice::{VoiceState, VoiceStateMove};
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
    post,
    path = "/voice/{channel_id}/move",
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

/// Voice state disconnect
#[endpoint(
    delete,
    path = "/voice/{channel_id}/member/{user_id}",
    tags = ["voice"],
    scopes = [Full],
    permissions_optional = [VoiceMove],
    audit_log_events = ["MemberDisconnect"],
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
    audit_log_events = ["MemberDisconnectAll"],
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
///
/// list all voice states in this channel
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

// ========== calls ==========

/// Voice call create
#[endpoint(
    post,
    path = "/voice/{channel_id}/call",
    tags = ["voice"],
    scopes = [Full],
    response(CREATED, body = Call, description = "ok"),
)]
pub mod voice_call_create {
    use crate::v1::types::voice::{Call, CallCreate};
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[json]
        pub call: CallCreate,
    }

    pub struct Response {
        #[json]
        pub call: Call,
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
    response(OK, body = Call, description = "ok"),
)]
pub mod voice_call_patch {
    use crate::v1::types::voice::{Call, CallPatch};
    use crate::v1::types::ChannelId;

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[json]
        pub call: CallPatch,
    }

    pub struct Response {
        #[json]
        pub call: Call,
    }
}

/// Voice call get
#[endpoint(
    get,
    path = "/voice/{channel_id}/call",
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

// ========== ringing ==========

/// Voice ring start
///
/// Notifies people in a dm/gdm that there's a call. There must be an active call.
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
///
/// stop ringing channel participants
#[endpoint(
    post,
    path = "/voice/{channel_id}/ring/stop",
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
///
/// check if this channel can be rung
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
