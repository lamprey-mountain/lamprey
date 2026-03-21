use lamprey_macros::endpoint;

/// App create
#[endpoint(
    post,
    path = "/app",
    tags = ["application"],
    permissions = [ApplicationCreate],
    response(CREATED, body = Application, description = "success"),
)]
pub mod app_create {
    use crate::v1::types::application::{Application, ApplicationCreate};

    pub struct Request {
        #[json]
        pub application: ApplicationCreate,
    }

    pub struct Response {
        #[json]
        pub application: Application,
    }
}

/// App list
#[endpoint(
    get,
    path = "/app",
    tags = ["application"],
    response(OK, body = PaginationResponse<Application>, description = "success"),
)]
pub mod app_list {
    use crate::v1::types::application::Application;
    use crate::v1::types::{ApplicationId, PaginationQuery, PaginationResponse};

    pub struct Request {
        #[query]
        pub pagination: PaginationQuery<ApplicationId>,
    }

    pub struct Response {
        #[json]
        pub applications: PaginationResponse<Application>,
    }
}

/// App get
#[endpoint(
    get,
    path = "/app/{app_id}",
    tags = ["application"],
    response(OK, body = Application, description = "success"),
)]
pub mod app_get {
    use crate::v1::types::application::Application;
    use crate::v1::types::misc::ApplicationIdReq;
    use crate::v1::types::ApplicationId;

    pub struct Request {
        #[path]
        pub app_id: ApplicationIdReq,
    }

    pub struct Response {
        #[json]
        pub application: Application,
    }
}

/// App patch
#[endpoint(
    patch,
    path = "/app/{app_id}",
    tags = ["application"],
    response(OK, body = Application, description = "success"),
)]
pub mod app_patch {
    use crate::v1::types::application::{Application, ApplicationPatch};
    use crate::v1::types::misc::ApplicationIdReq;
    use crate::v1::types::ApplicationId;

    pub struct Request {
        #[path]
        pub app_id: ApplicationIdReq,

        #[json]
        pub patch: ApplicationPatch,
    }

    pub struct Response {
        #[json]
        pub application: Application,
    }
}

/// App delete
#[endpoint(
    delete,
    path = "/app/{app_id}",
    tags = ["application"],
    response(NO_CONTENT, description = "success"),
)]
pub mod app_delete {
    use crate::v1::types::ApplicationId;

    pub struct Request {
        #[path]
        pub app_id: ApplicationId,
    }

    pub struct Response {}
}

/// App create session
#[endpoint(
    post,
    path = "/app/{app_id}/session",
    tags = ["application"],
    response(CREATED, body = SessionWithToken, description = "success"),
)]
pub mod app_create_session {
    use crate::v1::types::misc::ApplicationIdReq;
    use crate::v1::types::{SessionCreate, SessionWithToken};

    pub struct Request {
        #[path]
        pub app_id: ApplicationIdReq,

        #[json]
        pub session: SessionCreate,
    }

    pub struct Response {
        #[json]
        pub session: SessionWithToken,
    }
}

/// App invite bot
///
/// Add a bot to a room
#[endpoint(
    post,
    path = "/app/{app_id}/invite",
    tags = ["application"],
    permissions = [IntegrationsManage],
    response(NO_CONTENT, description = "success"),
)]
pub mod app_invite_bot {
    use crate::v1::types::{ApplicationId, RoomId};

    pub struct Request {
        #[path]
        pub app_id: ApplicationId,

        #[json]
        pub room_id: RoomId,
    }

    pub struct Response {}
}

/// Puppet ensure
#[endpoint(
    put,
    path = "/app/{app_id}/puppet/{puppet_id}",
    tags = ["application"],
    response(OK, body = User, description = "success"),
    response(CREATED, body = User, description = "created"),
)]
pub mod puppet_ensure {
    use crate::v1::types::misc::ApplicationIdReq;
    use crate::v1::types::{PuppetCreate, User};

    pub struct Request {
        #[path]
        pub app_id: ApplicationIdReq,

        #[path]
        pub puppet_id: String,

        #[json]
        pub puppet: PuppetCreate,
    }

    pub struct Response {
        #[json]
        pub user: User,
    }
}

/// App rotate oauth secret
#[endpoint(
    post,
    path = "/app/{app_id}/rotate-secret",
    tags = ["application"],
    response(OK, body = Application, description = "success"),
)]
pub mod app_rotate_secret {
    use crate::v1::types::application::Application;
    use crate::v1::types::misc::ApplicationIdReq;
    use crate::v1::types::ApplicationId;

    pub struct Request {
        #[path]
        pub app_id: ApplicationIdReq,
    }

    pub struct Response {
        #[json]
        pub application: Application,
    }
}
