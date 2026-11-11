use std::{
    collections::HashMap,
    fmt,
    io::{BufRead, BufReader, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use common::v1::types::auth::{PasswordExec, PasswordExecIdent, TotpVerificationRequest};
use common::v1::types::email::EmailAddr;
use common::v1::types::misc::SessionIdReq;
use common::v1::types::oauth::{
    OauthAuthMethod, OauthClientRegistrationRequest, OauthDeviceAuthorizationRequest,
    OauthGrantType, OauthTokenRequest,
};
use common::v1::types::{
    ApplicationId, Session, SessionCreate, SessionStatus, SessionToken, UserId,
};
use demand::{DemandOption, Input, Select};
use lamprey_sdk::http::Http;
use serde::Serialize;
use sha2::{Digest, Sha256};
use url::Url;

use crate::config::{CachedClient, Login, LyConfig};

/// how to authenticate when logging in
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display)]
pub enum AuthType {
    Token,
    PasswordUserId,
    PasswordEmail,
    DynamicClient,
}

/// the credentials obtained from an interactive login prompt
enum LoginCredentials {
    Token(SessionToken),
    Dynamic(DynamicLogin),
}

/// the result of a dynamic client registration login
struct DynamicLogin {
    token: SessionToken,
    refresh_token: Option<String>,
    client_id: ApplicationId,
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
    register: bool,
    scope: String,
    registration_token: Option<String>,
) -> Result<LoginResult> {
    if register && token.is_some() {
        bail!("--register cannot be combined with --token");
    }

    let api_url = prompt_api_url(api_url)?;
    let name = name.unwrap_or_else(|| "default".to_string());

    let credentials = match token {
        Some(token) => LoginCredentials::Token(SessionToken::from(token)),
        None if register => LoginCredentials::Dynamic(
            dynamic_login(config, &api_url, &scope, registration_token.as_deref()).await?,
        ),
        None => prompt_auth(config, &api_url, &scope, registration_token.as_deref()).await?,
    };

    let (token, refresh_token, client_id) = match credentials {
        LoginCredentials::Token(token) => (token, None, None),
        LoginCredentials::Dynamic(login) => {
            (login.token, login.refresh_token, Some(login.client_id))
        }
    };

    let session = authenticate(&api_url, &token).await?;
    let user_id = session
        .status
        .user_id()
        .context("session is not authenticated")?;

    let mut login = Login::new(name.clone(), api_url.clone(), token);
    login.refresh_token = refresh_token;
    login.client_id = client_id;
    config.add(login);
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

/// check whether a stored login is still valid, refreshing oauth logins if needed
pub async fn check(config: &mut LyConfig, name: Option<&str>) -> Result<CheckResult> {
    let mut login = config.login(name)?.clone();
    let session = match session_for_login(&login).await {
        Ok(session) => session,
        Err(err) => {
            let Some(refreshed) = refresh_login(&login).await? else {
                return Err(err).context("login is not valid");
            };

            login.token = refreshed.token;
            login.refresh_token = refreshed.refresh_token;
            config.add(login.clone());
            config.save()?;

            session_for_login(&login)
                .await
                .context("refreshed login is not valid")?
        }
    };

    Ok(CheckResult {
        name: login.name.clone(),
        api_url: login.api_url.clone(),
        session,
    })
}

async fn session_for_login(login: &Login) -> Result<Session> {
    let http = http_client(&login.api_url, Some(&login.token))?;
    http.session_get(SessionIdReq::SessionSelf).await
}

async fn refresh_login(login: &Login) -> Result<Option<DynamicLogin>> {
    let (Some(refresh_token), Some(client_id)) = (&login.refresh_token, login.client_id) else {
        return Ok(None);
    };

    let http = http_client(&login.api_url, None)?;
    let token = http
        .oauth_token(&OauthTokenRequest {
            grant_type: OauthGrantType::RefreshToken,
            code: None,
            redirect_uri: None,
            client_id: Some(client_id.to_string()),
            client_secret: None,
            refresh_token: Some(refresh_token.clone()),
            code_verifier: None,
            device_code: None,
            scope: None,
        })
        .await
        .context("failed to refresh login")?;

    Ok(Some(DynamicLogin {
        token: SessionToken::from(token.access_token),
        refresh_token: token.refresh_token,
        client_id,
    }))
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

fn prompt_api_url(api_url: Option<String>) -> Result<Url> {
    let api_url = match api_url {
        Some(api_url) => api_url,
        None => Input::new("api url")
            .placeholder("http://localhost:4000")
            .run()?,
    };

    Url::parse(&api_url).context("invalid api url")
}

fn session_name() -> String {
    format!("ly {}", env!("CARGO_PKG_VERSION"))
}

const CALLBACK_PATH: &str = "/callback";
const CALLBACK_TIMEOUT: Duration = Duration::from_secs(5 * 60);

/// loopback ports to register as redirect uris, tried in order
const CALLBACK_PORTS: [u16; 8] = [17321, 17322, 17323, 17324, 17325, 17326, 17327, 17328];

fn callback_uris() -> Vec<String> {
    CALLBACK_PORTS
        .iter()
        .map(|port| format!("http://127.0.0.1:{port}{CALLBACK_PATH}"))
        .collect()
}

fn random_bytes<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    common::util::random::fill(&mut bytes);
    bytes
}

/// register a dynamic client, then log in with the authorization code + pkce flow
async fn dynamic_login(
    config: &mut LyConfig,
    api_url: &Url,
    scope: &str,
    registration_token: Option<&str>,
) -> Result<DynamicLogin> {
    let http = http_client(api_url, None)?;
    let metadata = http
        .oauth_authorization_server()
        .await
        .context("failed to fetch oauth authorization server metadata")?;

    let client = match config.client(api_url) {
        Some(client) if cached_client_is_valid(api_url, client).await => client.clone(),
        _ => {
            let client = register_client(api_url, scope, registration_token).await?;
            config.set_client(client.clone());
            config.save()?;
            client
        }
    };
    let (listener, redirect_uri) = bind_callback(&client.redirect_uris)?;

    let verifier = BASE64_URL_SAFE_NO_PAD.encode(random_bytes::<32>());
    let challenge = BASE64_URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let state = BASE64_URL_SAFE_NO_PAD.encode(random_bytes::<16>());

    let mut url = metadata.authorization_endpoint;
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", &client.client_id.to_string())
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("scope", scope)
        .append_pair("state", &state)
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256");

    println!("opening browser to authorize...");
    if let Err(err) = webbrowser::open(url.as_str()) {
        eprintln!("warning: failed to open browser: {err}");
        println!("open this url to continue:\n{url}");
    }

    let code = tokio::task::spawn_blocking(move || wait_for_callback(listener, &state))
        .await
        .context("callback task panicked")??;

    let token = http
        .oauth_token(&OauthTokenRequest {
            grant_type: OauthGrantType::AuthorizationCode,
            code: Some(code),
            redirect_uri: Some(Url::parse(&redirect_uri)?),
            client_id: Some(client.client_id.to_string()),
            client_secret: None,
            refresh_token: None,
            code_verifier: Some(verifier),
            device_code: None,
            scope: None,
        })
        .await
        .context("failed to exchange authorization code")?;

    Ok(DynamicLogin {
        token: SessionToken::from(token.access_token),
        refresh_token: token.refresh_token,
        client_id: client.client_id,
    })
}

async fn cached_client_is_valid(api_url: &Url, client: &CachedClient) -> bool {
    let Ok(http) = http_client(api_url, None) else {
        return false;
    };

    // starting a device authorization confirms the cached client still exists
    http.oauth_device_start(&OauthDeviceAuthorizationRequest {
        client_id: client.client_id.to_string(),
        client_secret: None,
        scope: Some("identify".to_string()),
    })
    .await
    .is_ok()
}

async fn register_client(
    api_url: &Url,
    scope: &str,
    registration_token: Option<&str>,
) -> Result<CachedClient> {
    let token = registration_token.map(|token| SessionToken::from(token.to_string()));
    let http = http_client(api_url, token.as_ref())?;
    let redirect_uris = callback_uris();
    let client_name = session_name();

    let registration = http
        .oauth_register(&OauthClientRegistrationRequest {
            redirect_uris: redirect_uris.clone(),
            token_endpoint_auth_method: Some(OauthAuthMethod::None),
            grant_types: Some(vec![
                OauthGrantType::AuthorizationCode,
                OauthGrantType::RefreshToken,
            ]),
            response_types: None,
            client_name: Some(client_name.clone()),
            scope: Some(scope.to_string()),
        })
        .await
        .context("failed to register dynamic oauth client")?;

    Ok(CachedClient {
        api_url: api_url.clone(),
        client_id: registration.client_id,
        client_name,
        redirect_uris,
    })
}

fn bind_callback(redirect_uris: &[String]) -> Result<(TcpListener, String)> {
    let mut last_err = None;

    for uri in redirect_uris {
        let url = Url::parse(uri).with_context(|| format!("invalid redirect uri {uri}"))?;
        let Some(port) = url.port() else {
            continue;
        };
        let addr = SocketAddr::from(([127, 0, 0, 1], port));

        match TcpListener::bind(addr) {
            Ok(listener) => return Ok((listener, uri.clone())),
            Err(err) => last_err = Some(err),
        }
    }

    match last_err {
        Some(err) => Err(err).context("no loopback callback port is available"),
        None => bail!("the cached oauth client has no loopback redirect uris"),
    }
}

fn wait_for_callback(listener: TcpListener, expected_state: &str) -> Result<String> {
    listener.set_nonblocking(true)?;
    let deadline = Instant::now() + CALLBACK_TIMEOUT;

    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                if let Some(code) = handle_callback(stream, expected_state)? {
                    return Ok(code);
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    bail!("timed out waiting for browser authorization");
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(err) => return Err(err.into()),
        }
    }
}

fn handle_callback(mut stream: TcpStream, expected_state: &str) -> Result<Option<String>> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    if reader.read_line(&mut request_line)? == 0 {
        return Ok(None);
    }

    let mut parts = request_line.split_whitespace();
    let method = parts.next();
    let Some(target) = parts.next() else {
        respond(&mut stream, "400 Bad Request", "malformed request")?;
        return Ok(None);
    };

    if method != Some("GET") {
        respond(&mut stream, "405 Method Not Allowed", "method not allowed")?;
        return Ok(None);
    }

    let url = Url::parse(&format!("http://127.0.0.1{target}"))?;
    if url.path() != CALLBACK_PATH {
        respond(&mut stream, "404 Not Found", "not found")?;
        return Ok(None);
    }

    let params: HashMap<String, String> = url.query_pairs().into_owned().collect();
    if params.get("state").map(String::as_str) != Some(expected_state) {
        respond(&mut stream, "400 Bad Request", "state mismatch")?;
        return Ok(None);
    }

    if let Some(error) = params.get("error") {
        let description = params
            .get("error_description")
            .map(String::as_str)
            .unwrap_or("authorization failed");
        respond(
            &mut stream,
            "400 Bad Request",
            "authorization failed, check the terminal",
        )?;
        bail!("{error}: {description}");
    }

    let Some(code) = params.get("code") else {
        respond(&mut stream, "400 Bad Request", "missing authorization code")?;
        bail!("callback is missing the authorization code");
    };

    respond(
        &mut stream,
        "200 OK",
        "you can close this window and return to the terminal",
    )?;
    Ok(Some(code.clone()))
}

fn respond(stream: &mut TcpStream, status: &str, message: &str) -> Result<()> {
    let body = format!("<!doctype html><html><body><p>{message}</p></body></html>");
    write!(
        stream,
        "HTTP/1.1 {status}\r\ncontent-type: text/html; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    )?;
    stream.flush()?;
    Ok(())
}

async fn prompt_auth(
    config: &mut LyConfig,
    api_url: &Url,
    scope: &str,
    registration_token: Option<&str>,
) -> Result<LoginCredentials> {
    let auth_type = Select::new("auth type")
        .description("how do you want to authenticate to the server")
        .option(DemandOption::new(AuthType::Token).label("token"))
        .option(DemandOption::new(AuthType::PasswordUserId).label("password + user id"))
        .option(DemandOption::new(AuthType::PasswordEmail).label("password + email"))
        .option(
            DemandOption::new(AuthType::DynamicClient)
                .label("dynamic client registration (browser)"),
        )
        .run()?;

    match auth_type {
        AuthType::Token => Ok(LoginCredentials::Token(prompt_token()?)),
        AuthType::PasswordUserId | AuthType::PasswordEmail => Ok(LoginCredentials::Token(
            password_login(api_url, auth_type).await?,
        )),
        AuthType::DynamicClient => Ok(LoginCredentials::Dynamic(
            dynamic_login(config, api_url, scope, registration_token).await?,
        )),
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
        AuthType::Token | AuthType::DynamicClient => {
            bail!("this auth method does not use a password")
        }
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
