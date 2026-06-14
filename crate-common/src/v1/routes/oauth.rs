use lamprey_macros::endpoint;

/// Oauth info
///
/// Fetch information about an OAuth application before authorization.
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
///
/// Grant an application access to some resources.
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
/// Exchange an authorization token for an access token.
#[endpoint(
    post,
    path = "/oauth/token",
    tags = ["oauth"],
    response(OK, body = OauthTokenResponse, description = "success"),
)]
pub mod oauth_token {
    use crate::v1::types::oauth::{OauthTokenRequest, OauthTokenResponse};

    pub struct Request {
        #[form]
        pub token: OauthTokenRequest,
    }

    pub struct Response {
        #[json]
        pub token: OauthTokenResponse,
    }
}

/// Oauth introspect
///
/// Validate an access token and retrieve its associated information and scopes.
#[endpoint(
    post,
    path = "/oauth/introspect",
    tags = ["oauth"],
    response(OK, body = OauthIntrospectResponse, description = "success"),
)]
pub mod oauth_introspect {
    use crate::v1::types::oauth::OauthIntrospectResponse;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, serde::Serialize)]
    #[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
    pub struct IntrospectForm {
        pub token: String,
    }

    pub struct Request {
        #[form]
        pub form: IntrospectForm,
    }

    pub struct Response {
        #[json]
        pub introspect: OauthIntrospectResponse,
    }
}

/// Oauth userinfo
///
/// Retrieve profile information about the user currently authorized by an access token.
#[endpoint(
    get,
    path = "/oauth/userinfo",
    tags = ["oauth"],
    scopes = [Identify],
    response(OK, body = Userinfo, description = "success"),
)]
pub mod oauth_userinfo {
    use crate::v1::types::oauth::Userinfo;

    pub struct Request {}

    pub struct Response {
        #[json]
        pub userinfo: Userinfo,
    }
}

/// Oauth revoke
///
/// Invalidate an active access or refresh token, terminating the application's access.
#[endpoint(
    post,
    path = "/oauth/revoke",
    tags = ["oauth"],
    response(NO_CONTENT, description = "success"),
)]
pub mod oauth_revoke {
    use serde::Deserialize;

    #[derive(Debug, Deserialize, serde::Serialize)]
    #[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
    pub struct RevokeForm {
        pub token: String,
    }

    pub struct Request {
        #[form]
        pub form: RevokeForm,
    }

    pub struct Response {}
}

/// Oauth autoconfig
///
/// Retrieve the OpenID Connect discovery document for automatic client configuration.
// NOTE: should this be at the root? nesting it under `/api/v1` feels wrong but seems to work.
#[endpoint(
    get,
    path = "/.well-known/oauth-authorization-server",
    tags = ["oauth"],
    response(OK, body = Autoconfig, description = "success"),
)]
pub mod oauth_autoconfig {
    use crate::v1::types::oauth::Autoconfig;

    pub struct Request {}

    pub struct Response {
        #[json]
        pub autoconfig: Autoconfig,
    }
}
