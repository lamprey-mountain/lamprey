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

/// Oauth register
///
/// Register a new OAuth client (RFC 7591).
#[endpoint(
    post,
    path = "/oauth/register",
    tags = ["oauth"],
    response(CREATED, body = OauthClientRegistrationResponse, description = "registered"),
)]
pub mod oauth_register {
    use crate::v1::types::oauth::{
        OauthClientRegistrationRequest, OauthClientRegistrationResponse,
    };

    pub struct Request {
        #[json]
        pub registration: OauthClientRegistrationRequest,
    }

    pub struct Response {
        #[json]
        pub registration: OauthClientRegistrationResponse,
    }
}

/// Oauth device authorization
///
/// Start the device authorization flow.
#[endpoint(
    post,
    path = "/oauth/device/start",
    tags = ["oauth"],
    response(OK, body = OauthDeviceAuthorizationResponse, description = "success"),
)]
pub mod oauth_device_authorization {
    use crate::v1::types::oauth::{
        OauthDeviceAuthorizationRequest, OauthDeviceAuthorizationResponse,
    };

    pub struct Request {
        #[form]
        pub form: OauthDeviceAuthorizationRequest,
    }

    pub struct Response {
        #[json]
        pub device: OauthDeviceAuthorizationResponse,
    }
}

/// Oauth device info
///
/// Fetch information about a pending device authorization.
#[endpoint(
    get,
    path = "/oauth/device",
    tags = ["oauth"],
    scopes = [Identify],
    response(OK, body = OauthDeviceInfo, description = "success"),
)]
pub mod oauth_device_info {
    use crate::v1::types::oauth::{OauthDeviceInfo, OauthDeviceQuery};

    pub struct Request {
        #[query]
        pub query: OauthDeviceQuery,
    }

    pub struct Response {
        #[json]
        pub info: OauthDeviceInfo,
    }
}

/// Oauth device authorize
///
/// Approve or deny a pending device authorization.
#[endpoint(
    post,
    path = "/oauth/device/authorize",
    tags = ["oauth"],
    scopes = [Identify],
    response(NO_CONTENT, description = "success"),
)]
pub mod oauth_device_authorize {
    use crate::v1::types::oauth::OauthDeviceDecision;

    pub struct Request {
        #[json]
        pub decision: OauthDeviceDecision,
    }

    pub struct Response {}
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
    use crate::v1::types::oauth::OauthTokenRevoke;

    pub struct Request {
        #[form]
        pub form: OauthTokenRevoke,
    }

    pub struct Response {}
}

/// Oauth config authorization server
///
/// Retrieve the OpenID Connect discovery document for automatic client configuration.
// NOTE: should this be at the root? nesting it under `/api/v1` feels wrong but seems to work.
#[endpoint(
    get,
    path = "/.well-known/oauth-authorization-server",
    tags = ["oauth"],
    response(OK, body = AuthServerMetadata, description = "success"),
)]
pub mod oauth_autoconfig_auth_server {
    use crate::v1::types::oauth::AuthServerMetadata;

    pub struct Request {}

    pub struct Response {
        #[json]
        pub autoconfig: AuthServerMetadata,
    }
}

/// Oauth config OpenID Connect
///
/// Retrieve the OpenID Connect discovery document.
#[endpoint(
    get,
    path = "/.well-known/openid-configuration",
    tags = ["oauth"],
    response(OK, body = OidcDiscovery , description = "success"),
)]
pub mod oauth_autoconfig_openid {
    use crate::v1::types::oauth::OidcDiscovery;

    pub struct Request {}

    pub struct Response {
        #[json]
        pub config: OidcDiscovery,
    }
}

/// Oauth JWKS
///
/// Retrieve the JSON Web Key Set.
#[endpoint(
    get,
    path = "/.well-known/jwks.json",
    tags = ["oauth"],
    response(OK, body = Jwks, description = "success"),
)]
pub mod oauth_jwks {
    use crate::v1::types::oauth::Jwks;

    pub struct Request {}

    pub struct Response {
        #[json]
        pub jwks: Jwks,
    }
}
