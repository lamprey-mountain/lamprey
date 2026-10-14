use std::time::{Duration, Instant};

use common::v1::types::SessionId;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::oauth::{OauthGrantType, OauthTokenRequest, OauthTokenResponse};
use dashmap::DashMap;
use http::HeaderValue;
use kerosene_core::config::ConfigOauthProviderKind;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use crate::prelude::*;

// TODO: refactor and use RevokeForm in crate-common/src/v1/routes/oauth.rs (rename to OauthTokenRevoke, add field, etc)
#[derive(Debug, Serialize, Deserialize)]
pub struct OauthTokenRevoke {
    pub token_type_hint: String,
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiscordAuth {
    // NOTE: i'm assuming that `user` always exists for now
    /// the user who has authorized, if the user has authorized with the identify scope
    pub user: DiscordUser,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiscordUser {
    /// the user's id
    pub id: String,

    /// the user's username, not unique across the platform
    pub username: String,

    /// the user's display name, if it is set. For bots, this is the application name
    pub global_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GithubUser {
    /// the user's id
    pub id: u64,

    /// the user's name
    pub name: Option<String>,

    /// the user's username
    pub login: String,

    /// the user's bio
    pub bio: Option<String>,
}

// TODO: use Userinfo in common oauth
/// openid connect userinfo response
#[derive(Debug, Serialize, Deserialize)]
pub struct OidcUser {
    /// the subject identifier, unique within the issuer
    pub sub: String,

    /// the user's full name
    #[serde(default)]
    pub name: Option<String>,

    /// the user's preferred username
    #[serde(default)]
    pub preferred_username: Option<String>,

    /// the user's email address (requires the `email` scope)
    #[serde(default)]
    pub email: Option<String>,

    /// whether the email has been verified
    #[serde(default)]
    pub email_verified: Option<bool>,

    /// url to the user's avatar image
    #[serde(default)]
    pub picture: Option<String>,
}

pub struct OauthState {
    provider: String,
    session_id: SessionId,
    created_at: Instant,
}

impl OauthState {
    pub fn new(provider: String, session_id: SessionId) -> Self {
        Self {
            provider,
            session_id,
            created_at: Instant::now(),
        }
    }

    pub fn is_expired(&self, duration: Duration) -> bool {
        self.created_at.elapsed() > duration
    }
}

pub struct ServiceOauth {
    state: Globals,
    oauth_states: Arc<DashMap<Uuid, OauthState>>,
}

// TODO: use pkce/code_verifier s256 for oidc (and other oauth providers too?)
impl ServiceOauth {
    pub fn new(state: Globals) -> Self {
        let s = Self {
            state,
            oauth_states: Arc::new(DashMap::new()),
        };

        // clean up expired oauth states
        let s_clone = s.oauth_states.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;
                s_clone.retain(|_, state| !state.is_expired(Duration::from_secs(60 * 5)));
            }
        });

        s
    }

    pub fn create_url(&self, provider: &str, session_id: SessionId) -> Result<Url> {
        let p = self
            .state
            .config()
            .oauth_provider
            .get(provider)
            .ok_or(ApiError::from_code(ErrorCode::UnknownOauth2Client))?;
        let state = Uuid::new_v4();
        self.oauth_states
            .insert(state, OauthState::new(provider.to_string(), session_id));
        let redirect_uri: Url = self
            .state
            .config()
            .api_url
            .join(&format!("/api/v1/auth/oauth/{}/redirect", provider))?;
        let url = Url::parse_with_params(
            &p.authorization_url,
            [
                ("client_id", p.client_id.as_str()),
                ("redirect_uri", redirect_uri.as_str()),
                ("response_type", "code"),
                ("scope", p.scope()),
                ("state", &state.to_string()),
            ],
        )?;
        Ok(url)
    }

    pub async fn exchange_code_for_token(
        &self,
        state: Uuid,
        code: String,
    ) -> Result<(OauthTokenResponse, SessionId)> {
        const OAUTH_STATE_EXPIRATION: Duration = Duration::from_secs(60 * 5);

        let (_, s) = self
            .oauth_states
            .remove(&state)
            .ok_or(Error::BadStatic("invalid or expired state"))?;

        if s.is_expired(OAUTH_STATE_EXPIRATION) {
            return Err(Error::BadStatic("invalid or expired state"));
        }
        let client = reqwest::Client::new();
        let p = self
            .state
            .config()
            .oauth_provider
            .get(&s.provider)
            .ok_or(ApiError::from_code(ErrorCode::UnknownOauth2Client))?;
        let redirect_uri: Url = self
            .state
            .config()
            .api_url
            .join(&format!("/api/v1/auth/oauth/{}/redirect", s.provider))?;

        let ua = match p.kind {
            ConfigOauthProviderKind::Discord => HeaderValue::from_static(
                // TODO: get version from env at compile time
                "DiscordBot (https://git.celery.eu.org/lamprey/lamprey, 0.1.0)",
            ),
            _ => self
                .state
                .config()
                .user_agent_header_value()
                .map_err(|err| Error::Internal(err.to_string()))?,
        };

        let client_secret = p
            .client_secret
            .load()
            .map_err(|err| Error::Internal(err.to_string()))?
            .to_string();
        let (use_basic_auth, body) = match p.kind {
            ConfigOauthProviderKind::Github => (
                false,
                OauthTokenRequest {
                    grant_type: OauthGrantType::AuthorizationCode,
                    code: Some(code),
                    redirect_uri: Some(redirect_uri),
                    client_id: Some(p.client_id.clone()),
                    client_secret: Some(client_secret.clone()),
                    refresh_token: None,
                    code_verifier: None,
                },
            ),
            // assume everything else uses http basic auth for now
            // FIXME: this is probably not true at all
            // maybe redo the oauth provider config struct?
            _ => (
                true,
                OauthTokenRequest {
                    grant_type: OauthGrantType::AuthorizationCode,
                    code: Some(code),
                    redirect_uri: Some(redirect_uri),
                    client_id: None,
                    client_secret: None,
                    refresh_token: None,
                    code_verifier: None,
                },
            ),
        };

        let req = client
            .post(&p.token_url)
            .header("Accept", "application/json")
            .header("User-Agent", ua)
            .form(&body);

        let req = if use_basic_auth {
            req.basic_auth(&p.client_id, Some(&client_secret))
        } else {
            req
        };

        let res: OauthTokenResponse = req.send().await?.error_for_status()?.json().await?;

        Ok((res, s.session_id))
    }

    pub async fn revoke_token(&self, provider: &str, token: String) -> Result<()> {
        let p = self
            .state
            .config()
            .oauth_provider
            .get(provider)
            .ok_or(ApiError::from_code(ErrorCode::UnknownOauth2Client))?;
        let client = reqwest::Client::new();
        let body = OauthTokenRevoke {
            token_type_hint: "access_token".to_string(),
            token,
        };
        let client_secret = p
            .client_secret
            .load()
            .map_err(|err| Error::Internal(err.to_string()))?
            .to_string();
        client
            .post(&p.revocation_url)
            .basic_auth(&p.client_id, Some(&client_secret))
            .form(&body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn discord_get_user(&self, token: String) -> Result<DiscordAuth> {
        let client = reqwest::Client::new();
        let res: DiscordAuth = client
            .get("https://discord.com/api/v10/oauth2/@me")
            .header(
                "User-Agent",
                self.state
                    .config()
                    .user_agent_header_value()
                    .map_err(|err| Error::Internal(err.to_string()))?,
            )
            .bearer_auth(token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(res)
    }

    pub async fn github_get_user(&self, token: String) -> Result<GithubUser> {
        let client = reqwest::Client::new();
        let res: GithubUser = client
            .get("https://api.github.com/user")
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header(
                "User-Agent",
                self.state
                    .config()
                    .user_agent_header_value()
                    .map_err(|err| Error::Internal(err.to_string()))?,
            )
            .bearer_auth(token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(res)
    }

    /// fetch the profile of an openid connect user via the provider's userinfo endpoint
    pub async fn oidc_get_user(&self, provider: &str, token: String) -> Result<OidcUser> {
        // TODO: let users pass a well known autoconfig url
        // TODO: make lint() check for userinfo_url
        let p = self
            .state
            .config()
            .oauth_provider
            .get(provider)
            .ok_or(ApiError::from_code(ErrorCode::UnknownOauth2Client))?;
        let userinfo_url = p
            .userinfo_url
            .as_deref()
            .ok_or_else(|| Error::Internal("oauth provider is missing userinfo_url".into()))?;
        let client = reqwest::Client::new();
        let res: OidcUser = client
            .get(userinfo_url)
            .header("Accept", "application/json")
            .header(
                "User-Agent",
                self.state
                    .config()
                    .user_agent_header_value()
                    .map_err(|err| Error::Internal(err.to_string()))?,
            )
            .bearer_auth(token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(res)
    }
}
