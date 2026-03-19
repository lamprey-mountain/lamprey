use lamprey_macros::endpoint;

/// Session create
#[endpoint(
    post,
    path = "/session",
    tags = ["session"],
    response(CREATED, body = SessionWithToken, description = "success"),
)]
pub mod session_create {
    use crate::v1::types::{SessionCreate, SessionWithToken};

    pub struct Request {
        #[json]
        pub session: SessionCreate,
    }

    pub struct Response {
        #[json]
        pub session: SessionWithToken,
    }
}

/// Session list
#[endpoint(
    get,
    path = "/session",
    tags = ["session"],
    scopes = [Full],
    response(OK, body = PaginationResponse<Session>, description = "List session success"),
)]
pub mod session_list {
    use crate::v1::types::{PaginationQuery, PaginationResponse, Session, SessionId};

    pub struct Request {
        #[query]
        pub pagination: PaginationQuery<SessionId>,
    }

    pub struct Response {
        #[json]
        pub sessions: PaginationResponse<Session>,
    }
}

/// Session update
#[endpoint(
    patch,
    path = "/session/{session_id}",
    tags = ["session"],
    scopes = [Full],
    response(OK, body = Session, description = "success"),
    response(NOT_MODIFIED, body = Session, description = "not modified"),
)]
pub mod session_update {
    use crate::v1::types::{Session, SessionId, SessionPatch};
    use crate::types::SessionIdReq;

    pub struct Request {
        #[path]
        pub session_id: SessionIdReq,

        #[json]
        pub patch: SessionPatch,
    }

    pub struct Response {
        #[json]
        pub session: Session,
    }
}

/// Session delete
#[endpoint(
    delete,
    path = "/session/{session_id}",
    tags = ["session"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod session_delete {
    use crate::v1::types::SessionId;
    use crate::types::SessionIdReq;

    pub struct Request {
        #[path]
        pub session_id: SessionIdReq,
    }

    pub struct Response;
}

/// Session get
#[endpoint(
    get,
    path = "/session/{session_id}",
    tags = ["session"],
    scopes = [Full],
    response(OK, body = Session, description = "success"),
)]
pub mod session_get {
    use crate::v1::types::{Session, SessionId};
    use crate::types::SessionIdReq;

    pub struct Request {
        #[path]
        pub session_id: SessionIdReq,
    }

    pub struct Response {
        #[json]
        pub session: Session,
    }
}

/// Session status set
#[endpoint(
    put,
    path = "/session/{session_id}/status",
    tags = ["session"],
    scopes = [Full],
    response(OK, body = Session, description = "success"),
)]
pub mod session_status_set {
    use crate::v1::types::{Session, SessionId, SessionStatus};
    use crate::types::SessionIdReq;

    pub struct Request {
        #[path]
        pub session_id: SessionIdReq,

        #[json]
        pub status: SessionStatus,
    }

    pub struct Response {
        #[json]
        pub session: Session,
    }
}
