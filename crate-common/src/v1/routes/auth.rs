use lamprey_macros::endpoint;

/// Auth oauth init
#[endpoint(
    post,
    path = "/auth/oauth/{provider}",
    tags = ["auth"],
    response(OK, body = OauthInitResponse, description = "ready"),
)]
pub mod auth_oauth_init {
    pub use url::Url;

    pub struct Request {
        #[path]
        pub provider: String,
    }

    pub struct Response {
        #[json]
        pub oauth: OauthInitResponse,
    }
}

/// Oauth init response
#[derive(Debug, serde::Serialize)]
pub struct OauthInitResponse {
    pub url: url::Url,
}

/// Auth oauth redirect
#[endpoint(
    get,
    path = "/auth/oauth/{provider}/redirect",
    tags = ["auth"],
    response(OK, description = "success; responds with html + javascript"),
)]
pub mod auth_oauth_redirect {
    pub struct Request {
        #[path]
        pub provider: String,

        #[query]
        pub state: String,

        #[query]
        pub code: String,
    }

    pub struct Response {}
}

/// Auth register
#[endpoint(
    post,
    path = "/auth/register",
    tags = ["auth"],
    response(OK, body = SessionWithToken, description = "success"),
)]
pub mod auth_register {
    use crate::v1::types::{SessionWithToken, UserCreate};

    pub struct Request {
        #[json]
        pub register: UserCreate,
    }

    pub struct Response {
        #[json]
        pub session: SessionWithToken,
    }
}

/// Auth login
#[endpoint(
    post,
    path = "/auth/login",
    tags = ["auth"],
    response(OK, body = SessionWithToken, description = "success"),
)]
pub mod auth_login {
    use crate::v1::types::SessionWithToken;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub struct LoginRequest {
        pub username: String,
        pub password: String,
    }

    pub struct Request {
        #[json]
        pub login: LoginRequest,
    }

    pub struct Response {
        #[json]
        pub session: SessionWithToken,
    }
}

/// Auth logout
#[endpoint(
    post,
    path = "/auth/logout",
    tags = ["auth"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod auth_logout {
    pub struct Request {}
    pub struct Response {}
}

/// Auth totp init
#[endpoint(
    post,
    path = "/auth/totp/init",
    tags = ["auth"],
    scopes = [Full],
    response(OK, body = TotpInit, description = "success"),
)]
pub mod auth_totp_init {
    use crate::v1::types::auth::TotpInit;

    pub struct Request {}

    pub struct Response {
        #[json]
        pub totp: TotpInit,
    }
}

/// Auth totp enable
#[endpoint(
    post,
    path = "/auth/totp/enable",
    tags = ["auth"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod auth_totp_enable {
    use crate::v1::types::auth::TotpVerificationRequest;

    pub struct Request {
        #[json]
        pub verification: TotpVerificationRequest,
    }

    pub struct Response {}
}

/// Auth totp recovery codes get
///
/// View existing recovery codes (does not invalidate them)
#[endpoint(
    get,
    path = "/auth/totp/recovery-codes",
    tags = ["auth"],
    scopes = [Full],
    response(OK, body = TotpRecoveryCodes, description = "success"),
)]
pub mod auth_totp_recovery_codes_get {
    use crate::v1::types::auth::TotpRecoveryCodes;

    pub struct Request {}

    pub struct Response {
        #[json]
        pub codes: TotpRecoveryCodes,
    }
}

/// Auth totp recovery codes rotate
///
/// Generate new recovery codes (invalidates old ones)
#[endpoint(
    post,
    path = "/auth/totp/recovery-codes",
    tags = ["auth"],
    scopes = [Full],
    response(OK, body = TotpRecoveryCodes, description = "success"),
)]
pub mod auth_totp_recovery_codes_rotate {
    use crate::v1::types::auth::TotpRecoveryCodes;

    pub struct Request {}

    pub struct Response {
        #[json]
        pub codes: TotpRecoveryCodes,
    }
}

/// Auth password set
#[endpoint(
    put,
    path = "/auth/password",
    tags = ["auth"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod auth_password_set {
    use crate::v1::types::auth::PasswordSet;

    pub struct Request {
        #[json]
        pub password: PasswordSet,
    }

    pub struct Response {}
}

/// Auth password exec
#[endpoint(
    post,
    path = "/auth/password/exec",
    tags = ["auth"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod auth_password_exec {
    use crate::v1::types::auth::PasswordExec;

    pub struct Request {
        #[json]
        pub password: PasswordExec,
    }

    pub struct Response {}
}

/// Auth webauthn challenge
#[endpoint(
    post,
    path = "/auth/webauthn/challenge",
    tags = ["auth"],
    scopes = [Full],
    response(OK, body = WebauthnChallenge, description = "success"),
)]
pub mod auth_webauthn_challenge {
    use crate::v1::types::auth::WebauthnChallenge;

    pub struct Request {}

    pub struct Response {
        #[json]
        pub challenge: WebauthnChallenge,
    }
}

/// Auth webauthn finish
#[endpoint(
    post,
    path = "/auth/webauthn/finish",
    tags = ["auth"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod auth_webauthn_finish {
    use crate::v1::types::auth::WebauthnFinish;

    pub struct Request {
        #[json]
        pub finish: WebauthnFinish,
    }

    pub struct Response {}
}

/// Auth webauthn authenticators
#[endpoint(
    get,
    path = "/auth/webauthn/authenticator",
    tags = ["auth"],
    scopes = [Full],
    response(OK, body = Vec<WebauthnAuthenticator>, description = "success"),
)]
pub mod auth_webauthn_authenticators {
    use crate::v1::types::auth::WebauthnAuthenticator;

    pub struct Request {}

    pub struct Response {
        #[json]
        pub authenticators: Vec<WebauthnAuthenticator>,
    }
}

/// Auth webauthn authenticator delete
#[endpoint(
    delete,
    path = "/auth/webauthn/authenticator/{authenticator_id}",
    tags = ["auth"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod auth_webauthn_authenticator_delete {
    pub struct Request {
        #[path]
        pub authenticator_id: String,
    }

    pub struct Response {}
}

/// Auth captcha challenge
#[endpoint(
    get,
    path = "/auth/captcha",
    tags = ["auth"],
    response(OK, body = CaptchaChallenge, description = "success"),
)]
pub mod auth_captcha_challenge {
    use crate::v1::types::auth::CaptchaChallenge;

    pub struct Request {}

    pub struct Response {
        #[json]
        pub captcha: CaptchaChallenge,
    }
}

/// Auth oauth delete
///
/// Remove an oauth provider
#[endpoint(
    delete,
    path = "/auth/oauth/{provider}",
    tags = ["auth"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod auth_oauth_delete {
    pub struct Request {
        #[path]
        pub provider: String,
    }

    pub struct Response {}
}

/// Auth email exec
///
/// Send a magic link email to login
#[endpoint(
    post,
    path = "/auth/email/{addr}",
    tags = ["auth"],
    response(ACCEPTED, description = "success"),
)]
pub mod auth_email_exec {
    pub struct Request {
        #[path]
        pub addr: String,
    }

    pub struct Response {}
}

/// Auth email reset
///
/// Send a password reset email
#[endpoint(
    post,
    path = "/auth/email/{addr}/reset",
    tags = ["auth"],
    response(ACCEPTED, description = "success"),
)]
pub mod auth_email_reset {
    pub struct Request {
        #[path]
        pub addr: String,
    }

    pub struct Response {}
}

/// Auth email complete
///
/// Complete email authentication
#[endpoint(
    post,
    path = "/auth/email/{addr}/complete",
    tags = ["auth"],
    response(NO_CONTENT, description = "success"),
)]
pub mod auth_email_complete {
    use crate::v1::types::auth::AuthEmailComplete;

    pub struct Request {
        #[path]
        pub addr: String,

        #[json]
        pub complete: AuthEmailComplete,
    }

    pub struct Response {}
}

/// Auth totp exec
///
/// Execute totp authentication
#[endpoint(
    post,
    path = "/auth/totp",
    tags = ["auth"],
    response(OK, body = AuthState, description = "success"),
)]
pub mod auth_totp_exec {
    use crate::v1::types::auth::{AuthState, TotpVerificationRequest};

    pub struct Request {
        #[json]
        pub verification: TotpVerificationRequest,
    }

    pub struct Response {
        #[json]
        pub state: AuthState,
    }
}

/// Auth totp recovery exec
///
/// Use a recovery code
#[endpoint(
    post,
    path = "/auth/totp/recovery",
    tags = ["auth"],
    response(OK, body = AuthState, description = "success"),
)]
pub mod auth_totp_recovery_exec {
    use crate::v1::types::auth::{AuthState, TotpVerificationRequest};

    pub struct Request {
        #[json]
        pub verification: TotpVerificationRequest,
    }

    pub struct Response {
        #[json]
        pub state: AuthState,
    }
}

/// Auth totp delete
///
/// Delete totp configuration
#[endpoint(
    delete,
    path = "/auth/totp",
    tags = ["auth"],
    scopes = [Full],
    response(OK, body = AuthState, description = "success"),
)]
pub mod auth_totp_delete {
    use crate::v1::types::auth::AuthState;

    pub struct Request {}

    pub struct Response {
        #[json]
        pub state: AuthState,
    }
}

/// Auth password delete
///
/// Remove password authentication
#[endpoint(
    delete,
    path = "/auth/password",
    tags = ["auth"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod auth_password_delete {
    pub struct Request {}

    pub struct Response {}
}

/// Auth state
///
/// Get the available auth methods for this user
#[endpoint(
    get,
    path = "/auth/state",
    tags = ["auth"],
    scopes = [Full],
    response(OK, body = AuthState, description = "success"),
)]
pub mod auth_state {
    use crate::v1::types::auth::AuthState;

    pub struct Request {}

    pub struct Response {
        #[json]
        pub state: AuthState,
    }
}
