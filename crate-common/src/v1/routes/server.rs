use lamprey_macros::endpoint;

/// Server information
#[endpoint(
    get,
    path = "/server/@self",
    tags = ["server"],
    scopes = [Full],
    response(OK, body = ServerInfo, description = "Get server info success"),
)]
pub mod server_info {
    use crate::v1::types::server::ServerInfo;

    pub struct Request {}

    pub struct Response {
        #[json]
        pub server: ServerInfo,
    }
}

/// Server moderation
#[endpoint(
    get,
    path = "/server/@self/moderation",
    tags = ["server"],
    scopes = [Full],
    response(OK, body = ServerModeration, description = "Get server moderation capabilities success"),
)]
pub mod server_moderation {
    use crate::v1::types::server::ServerModeration;

    pub struct Request {}

    pub struct Response {
        #[json]
        pub moderation: ServerModeration,
    }
}

/// Server voice
#[endpoint(
    get,
    path = "/server/@self/voice",
    tags = ["server"],
    scopes = [Full],
    permissions = [Admin],
    response(OK, body = Vec<ServerVoiceSfu>, description = "Get server voice sfus success"),
)]
pub mod server_voice {
    use crate::v1::types::server::ServerVoiceSfu;

    pub struct Request {}

    pub struct Response {
        #[json]
        pub voice: Vec<ServerVoiceSfu>,
    }
}
