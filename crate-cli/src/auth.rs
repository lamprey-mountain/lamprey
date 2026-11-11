use std::fmt;

use anyhow::{Context, Result, bail};
use common::v1::types::auth::{PasswordExec, PasswordExecIdent, TotpVerificationRequest};
use common::v1::types::email::EmailAddr;
use common::v1::types::misc::SessionIdReq;
use common::v1::types::{Session, SessionCreate, SessionStatus, SessionToken, UserId};
use demand::{DemandOption, Input, Select};
use lamprey_sdk::http::Http;
use serde::Serialize;
use url::Url;

use crate::config::{Login, LyConfig};

/// how to authenticate when logging in
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display)]
pub enum AuthType {
    Token,
    PasswordUserId,
    PasswordEmail,
}

/// result of `ly auth login`
#[derive(Debug, Serialize)]
pub struct LoginResult {
    pub name: String,
    pub api_url: Url,
    pub user_id: UserId,
}

/// result of `ly auth logout`
#[derive(Debug, Serialize)]
pub struct LogoutResult {
    pub name: String,
    pub api_url: Url,
}

/// result of `ly auth check`
#[derive(Debug, Serialize)]
pub struct CheckResult {
    pub name: String,
    pub api_url: Url,
    pub session: Session,
}

/// result of `ly auth list`
#[derive(Debug, Serialize)]
pub struct ListResult {
    pub logins: Vec<ListedLogin>,
}

#[derive(Debug, Serialize)]
pub struct ListedLogin {
    pub name: String,
    pub api_url: Url,
    pub default: bool,
}

impl fmt::Display for LoginResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "logged in to {} as {} (user {})",
            self.api_url, self.name, self.user_id
        )
    }
}

impl fmt::Display for LogoutResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "logged out {} ({})", self.name, self.api_url)
    }
}

impl fmt::Display for CheckResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.session.name.as_deref().unwrap_or("<unnamed>");
        match self.session.status.user_id() {
            Some(user_id) => write!(
                f,
                "{}: {} ({}), user {user_id}, session {} ({name})",
                self.session.status, self.name, self.api_url, self.session.id
            ),
            None => write!(
                f,
                "{}: {} ({}), session {} ({name})",
                self.session.status, self.name, self.api_url, self.session.id
            ),
        }
    }
}

impl fmt::Display for ListResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.logins.is_empty() {
            return f.write_str("no logins");
        }

        let lines: Vec<_> = self
            .logins
            .iter()
            .map(|login| {
                let marker = if login.default { "*" } else { " " };
                format!("{marker} {} ({})", login.name, login.api_url)
            })
            .collect();

        f.write_str(&lines.join("\n"))
    }
}

/// login and store the resulting credentials
pub async fn login(
    config: &mut LyConfig,
    api_url: Option<String>,
    token: Option<String>,
    name: Option<String>,
) -> Result<LoginResult> {
    let api_url = match api_url {
        Some(api_url) => Url::parse(&api_url).context("invalid api url")?,
        None => {
            let api_url = Input::new("api url")
                .placeholder("http://localhost:4000")
                .run()?;
            Url::parse(&api_url).context("invalid api url")?
        }
    };

    let name = name.unwrap_or_else(|| "default".to_string());
    let token = match token {
        Some(token) => SessionToken::from(token),
        None => prompt_auth(&api_url).await?,
    };

    let session = authenticate(&api_url, &token).await?;
    let user_id = session
        .status
        .user_id()
        .context("session is not authenticated")?;

    config.add(Login::new(name.clone(), api_url.clone(), token));
    config.save()?;

    Ok(LoginResult {
        name,
        api_url,
        user_id,
    })
}

/// revoke the stored session, then remove the login from the config
pub async fn logout(config: &mut LyConfig, name: Option<&str>) -> Result<LogoutResult> {
    let login = config.login(name)?.clone();

    let revoke = async {
        let http = http_client(&login.api_url, Some(&login.token))?;
        http.session_delete(SessionIdReq::SessionSelf)
            .await
            .context("failed to revoke session")
    };
    if let Err(err) = revoke.await {
        eprintln!("warning: {err:#}");
    }

    config.remove(name)?;
    config.save()?;

    Ok(LogoutResult {
        name: login.name,
        api_url: login.api_url,
    })
}

/// check whether a stored login is still valid
pub async fn check(config: &LyConfig, name: Option<&str>) -> Result<CheckResult> {
    let login = config.login(name)?;
    let http = http_client(&login.api_url, Some(&login.token))?;
    let session = http
        .session_get(SessionIdReq::SessionSelf)
        .await
        .context("login is not valid")?;

    Ok(CheckResult {
        name: login.name.clone(),
        api_url: login.api_url.clone(),
        session,
    })
}

/// list the stored logins
pub fn list(config: &LyConfig) -> ListResult {
    let logins = config
        .logins
        .iter()
        .map(|login| ListedLogin {
            name: login.name.clone(),
            api_url: login.api_url.clone(),
            default: login.default,
        })
        .collect();

    ListResult { logins }
}

fn session_name() -> String {
    format!("ly {}", env!("CARGO_PKG_VERSION"))
}

async fn prompt_auth(api_url: &Url) -> Result<SessionToken> {
    let auth_type = Select::new("auth type")
        .description("how do you want to authenticate to the server")
        .option(DemandOption::new(AuthType::Token).label("token"))
        .option(DemandOption::new(AuthType::PasswordUserId).label("password + user id"))
        .option(DemandOption::new(AuthType::PasswordEmail).label("password + email"))
        .run()?;

    match auth_type {
        AuthType::Token => prompt_token(),
        AuthType::PasswordUserId | AuthType::PasswordEmail => {
            password_login(api_url, auth_type).await
        }
    }
}

fn prompt_token() -> Result<SessionToken> {
    let token = Input::new("token")
        .placeholder("secret-uuid-here")
        .password(true)
        .run()?;

    Ok(SessionToken::from(token))
}

/// create a session, then bind it to a user with a password
async fn password_login(api_url: &Url, auth_type: AuthType) -> Result<SessionToken> {
    let http = http_client(api_url, None)?;
    let session = http
        .session_create(&SessionCreate {
            name: Some(session_name()),
        })
        .await
        .context("failed to create a session")?;

    let ident = match auth_type {
        AuthType::PasswordUserId => PasswordExecIdent::UserId {
            user_id: Input::new("user id")
                .run()?
                .parse()
                .context("invalid user id")?,
        },
        AuthType::PasswordEmail => {
            let email = Input::new("email").run()?;
            PasswordExecIdent::Email {
                email: EmailAddr::try_from(email)
                    .map_err(|err| anyhow::anyhow!("invalid email: {err}"))?,
            }
        }
        AuthType::Token => bail!("token auth does not use a password"),
    };

    let password = Input::new("password").password(true).run()?;

    let token = session.token;
    let http = http_client(api_url, Some(&token))?;
    http.auth_password_exec(&PasswordExec { password, ident })
        .await
        .context("failed to log in")?;

    Ok(token)
}

/// validate a session token, completing a second factor if required
async fn authenticate(api_url: &Url, token: &SessionToken) -> Result<Session> {
    let http = http_client(api_url, Some(token))?;
    let session = http
        .session_get(SessionIdReq::SessionSelf)
        .await
        .context("failed to validate login")?;

    match &session.status {
        SessionStatus::Authorized { .. } | SessionStatus::Sudo { .. } => Ok(session),
        SessionStatus::Unauthorized => bail!("session is not authenticated"),
        SessionStatus::Bound { .. } => authenticate_second_factor(&http).await,
    }
}

async fn authenticate_second_factor(http: &Http) -> Result<Session> {
    let state = http
        .auth_state()
        .await
        .context("failed to fetch auth state")?;

    if !state.has_totp {
        if state.authenticators.is_empty() {
            bail!("this account requires a second factor, but no supported method is available");
        }

        bail!("this account only has webauthn authenticators, which ly does not support yet");
    }

    let code = Input::new("totp code").run()?;
    http.auth_totp_exec(&TotpVerificationRequest { code })
        .await
        .context("failed to verify totp code")?;

    http.session_get(SessionIdReq::SessionSelf)
        .await
        .context("failed to validate login")
}

fn http_client(api_url: &Url, token: Option<&SessionToken>) -> Result<Http> {
    let builder = Http::builder()
        .api_url(api_url.clone())
        .cdn_url(api_url.clone());

    match token {
        Some(token) => builder
            .token(token.clone())
            .build()
            .context("failed to build http client"),
        None => builder
            .build_unauthenticated()
            .context("failed to build http client"),
    }
}
