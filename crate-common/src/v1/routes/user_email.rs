use lamprey_macros::endpoint;

/// Email add
#[endpoint(
    put,
    path = "/user/{user_id}/email/{addr}",
    tags = ["user_email"],
    scopes = [Full],
    response(CREATED, description = "success"),
    response(OK, description = "already exists"),
)]
pub mod email_add {
    use crate::v1::types::misc::UserIdReq;

    pub struct Request {
        #[path]
        pub user_id: UserIdReq,

        #[path]
        pub addr: String,
    }

    pub struct Response;
}

/// Email delete
#[endpoint(
    delete,
    path = "/user/{user_id}/email/{addr}",
    tags = ["user_email"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod email_delete {
    use crate::v1::types::misc::UserIdReq;

    pub struct Request {
        #[path]
        pub user_id: UserIdReq,

        #[path]
        pub addr: String,
    }

    pub struct Response;
}

/// Email verify
#[endpoint(
    post,
    path = "/user/{user_id}/email/{addr}/verify",
    tags = ["user_email"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod email_verify {
    use crate::v1::types::misc::UserIdReq;

    pub struct Request {
        #[path]
        pub user_id: UserIdReq,

        #[path]
        pub addr: String,

        #[json]
        pub code: String,
    }

    pub struct Response;
}

/// Email set primary
#[endpoint(
    put,
    path = "/user/{user_id}/email/{addr}/primary",
    tags = ["user_email"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod email_set_primary {
    use crate::v1::types::misc::UserIdReq;

    pub struct Request {
        #[path]
        pub user_id: UserIdReq,

        #[path]
        pub addr: String,
    }

    pub struct Response;
}

/// Email list
#[endpoint(
    get,
    path = "/user/{user_id}/email",
    tags = ["user_email"],
    scopes = [Full],
    response(OK, body = Vec<EmailInfo>, description = "success"),
)]
pub mod email_list {
    use crate::v1::types::email::EmailInfo;
    use crate::v1::types::misc::UserIdReq;

    pub struct Request {
        #[path]
        pub user_id: UserIdReq,
    }

    pub struct Response {
        #[json]
        pub emails: Vec<EmailInfo>,
    }
}

/// Email update
#[endpoint(
    patch,
    path = "/user/{user_id}/email/{addr}",
    tags = ["user_email"],
    scopes = [Full],
    response(OK, body = EmailInfo, description = "success"),
)]
pub mod email_update {
    use crate::v1::types::email::EmailInfo;
    use crate::v1::types::email::EmailInfoPatch;
    use crate::v1::types::misc::UserIdReq;

    pub struct Request {
        #[path]
        pub user_id: UserIdReq,

        #[path]
        pub addr: String,

        #[json]
        pub patch: EmailInfoPatch,
    }

    pub struct Response {
        #[json]
        pub email: EmailInfo,
    }
}
