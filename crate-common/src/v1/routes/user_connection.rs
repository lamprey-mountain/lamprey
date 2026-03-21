use lamprey_macros::endpoint;

/// User connection list
#[endpoint(
    get,
    path = "/user/{user_id}/connection",
    tags = ["user_connection"],
    response(OK, body = PaginationResponse<Connection>, description = "success"),
)]
pub mod user_connection_list {
    use crate::v1::types::application::Connection;
    use crate::v1::types::misc::UserIdReq;
    use crate::v1::types::{ApplicationId, PaginationQuery, PaginationResponse};

    pub struct Request {
        #[path]
        pub user_id: UserIdReq,

        #[query]
        pub pagination: PaginationQuery<ApplicationId>,
    }

    pub struct Response {
        #[json]
        pub connections: PaginationResponse<Connection>,
    }
}

/// User connection update
#[endpoint(
    patch,
    path = "/user/{user_id}/connection/{app_id}",
    tags = ["user_connection"],
    response(OK, body = Connection, description = "success"),
)]
pub mod user_connection_update {
    use crate::v1::types::application::Connection;
    use crate::v1::types::misc::UserIdReq;
    use crate::v1::types::user_connection::ConnectionPatch;
    use crate::v1::types::ApplicationId;

    pub struct Request {
        #[path]
        pub user_id: UserIdReq,

        #[path]
        pub app_id: ApplicationId,

        #[json]
        pub patch: ConnectionPatch,
    }

    pub struct Response {
        #[json]
        pub connection: Connection,
    }
}

/// User connection delete
#[endpoint(
    delete,
    path = "/user/{user_id}/connection/{app_id}",
    tags = ["user_connection"],
    response(NO_CONTENT, description = "success"),
)]
pub mod user_connection_delete {
    use crate::v1::types::misc::UserIdReq;
    use crate::v1::types::ApplicationId;

    pub struct Request {
        #[path]
        pub user_id: UserIdReq,

        #[path]
        pub app_id: ApplicationId,
    }

    pub struct Response {}
}

/// User connection metadata get
#[endpoint(
    get,
    path = "/user/@self/app/{app_id}/connection-metadata",
    tags = ["user_connection"],
    response(OK, body = ConnectionMetadata, description = "success"),
)]
pub mod user_connection_metadata_get {
    use crate::v1::types::user_connection::ConnectionMetadata;
    use crate::v1::types::ApplicationId;

    pub struct Request {
        #[path]
        pub app_id: ApplicationId,
    }

    pub struct Response {
        #[json]
        pub metadata: ConnectionMetadata,
    }
}

/// User connection metadata put
#[endpoint(
    put,
    path = "/user/@self/app/{app_id}/connection-metadata",
    tags = ["user_connection"],
    response(OK, body = ConnectionMetadata, description = "success"),
)]
pub mod user_connection_metadata_put {
    use crate::v1::types::user_connection::ConnectionMetadata;
    use crate::v1::types::ApplicationId;

    pub struct Request {
        #[path]
        pub app_id: ApplicationId,

        #[json]
        pub metadata: ConnectionMetadata,
    }

    pub struct Response {
        #[json]
        pub metadata: ConnectionMetadata,
    }
}
