use lamprey_macros::endpoint;

/// Oauth info
#[endpoint(
    get,
    path = "/oauth/authorize",
    tags = ["oauth"],
    scopes = [Identify],
    response(OK, body = OauthAuthorizeInfo, description = "success"),
)]
pub mod oauth_info {
    use crate::v1::types::oauth::{OauthAuthorizeInfo, OauthAuthorizeParams};

    pub struct Request {
        #[query]
        pub params: OauthAuthorizeParams,
    }

    pub struct Response {
        #[json]
        pub info: OauthAuthorizeInfo,
    }
}

/// Oauth authorize
#[endpoint(
    post,
    path = "/oauth/authorize",
    tags = ["oauth"],
    scopes = [Identify],
    response(OK, body = OauthAuthorizeResponse, description = "success"),
)]
pub mod oauth_authorize {
    use crate::v1::types::oauth::{OauthAuthorizeParams, OauthAuthorizeResponse};

    pub struct Request {
        #[query]
        pub params: OauthAuthorizeParams,
    }

    pub struct Response {
        #[json]
        pub response: OauthAuthorizeResponse,
    }
}

/// Oauth token
///
/// Exchange an authorization token for an access token
#[endpoint(
    post,
    path = "/oauth/token",
    tags = ["oauth"],
    response(OK, body = OauthTokenResponse, description = "success"),
)]
pub mod oauth_token {
    use crate::v1::types::oauth::{OauthTokenRequest, OauthTokenResponse};

    pub struct Request {
        #[json]
        pub token: OauthTokenRequest,
    }

    pub struct Response {
        #[json]
        pub token: OauthTokenResponse,
    }
}

/// Oauth introspect
#[endpoint(
    post,
    path = "/oauth/introspect",
    tags = ["oauth"],
    response(OK, body = OauthIntrospectResponse, description = "success"),
)]
pub mod oauth_introspect {
    use crate::v1::types::oauth::OauthIntrospectResponse;

    pub struct Request {
        #[form]
        pub token: String,
    }

    pub struct Response {
        #[json]
        pub introspect: OauthIntrospectResponse,
    }
}

/// Oauth userinfo
#[endpoint(
    get,
    path = "/oauth/userinfo",
    tags = ["oauth"],
    scopes = [Identify],
    response(OK, body = Userinfo, description = "success"),
)]
pub mod oauth_userinfo {
    use crate::v1::types::oauth::Userinfo;

    pub struct Request;

    pub struct Response {
        #[json]
        pub userinfo: Userinfo,
    }
}

/// Oauth revoke
#[endpoint(
    post,
    path = "/oauth/revoke",
    tags = ["oauth"],
    response(NO_CONTENT, description = "success"),
)]
pub mod oauth_revoke {
    pub struct Request {
        #[form]
        pub token: String,
    }

    pub struct Response;
}

/// Oauth autoconfig
#[endpoint(
    get,
    path = "/.well-known/oauth-authorization-server",
    tags = ["oauth"],
    response(OK, body = Autoconfig, description = "success"),
)]
pub mod oauth_autoconfig {
    use crate::v1::types::oauth::Autoconfig;

    pub struct Request;

    pub struct Response {
        #[json]
        pub autoconfig: Autoconfig,
    }
}
