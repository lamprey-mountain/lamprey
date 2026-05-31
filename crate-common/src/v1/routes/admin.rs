use lamprey_macros::endpoint;

/// Admin whisper
#[endpoint(
    post,
    path = "/admin/whisper",
    tags = ["admin"],
    scopes = [Full],
    permissions_server = [Admin],
    audit_log_events = ["AdminWhisper"],
    response(ACCEPTED, description = "success"),
)]
pub mod admin_whisper {
    use crate::v1::types::admin::AdminWhisper;

    pub struct Request {
        #[json]
        pub body: AdminWhisper,
    }

    pub struct Response {}
}

/// Admin broadcast
#[endpoint(
    post,
    path = "/admin/broadcast",
    tags = ["admin"],
    scopes = [Full],
    permissions_server = [Admin],
    audit_log_events = ["AdminBroadcast"],
    response(ACCEPTED, description = "success"),
)]
pub mod admin_broadcast {
    use crate::v1::types::admin::AdminBroadcast;

    pub struct Request {
        #[json]
        pub body: AdminBroadcast,
    }

    pub struct Response {}
}

/// Admin register user
// TODO: make this POST /user/{user_id}/register
#[endpoint(
    post,
    path = "/admin/register-user",
    tags = ["admin"],
    scopes = [Full],
    permissions_server = [Admin],
    audit_log_events = ["UserRegistered"],
    response(ACCEPTED, description = "User registered"),
)]
pub mod admin_register_user {
    use crate::v1::types::admin::AdminRegisterUser;

    pub struct Request {
        #[json]
        pub body: AdminRegisterUser,
    }

    pub struct Response {}
}
