// TODO: port to https://docs.rs/oauth2/latest/oauth2/
// TODO: make more generic

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use common::v1::types::SessionId;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use crate::{
    error::{Error, Result},
    ServerStateInner,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct OauthTokenExchange {
    pub grant_type: String,
    pub code: String,
    pub redirect_uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OauthTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub scope: String,
}

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
    state: Arc<ServerStateInner>,
    oauth_states: Arc<DashMap<Uuid, OauthState>>,
}

impl ServiceOauth {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        let s = Self {
            state,
            oauth_states: Arc::new(DashMap::new()),
        };

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
            .config
            .oauth_provider
            .get(provider)
            .ok_or(Error::NotFound)?;
        let state = Uuid::new_v4();
        self.oauth_states
            .insert(state, OauthState::new(provider.to_string(), session_id));
        let redirect_uri: Url = self
            .state
            .config
            .api_url
            .join(&format!("/api/v1/auth/oauth/{}/redirect", provider))?;
        let url = Url::parse_with_params(
            &p.authorization_url,
            [
                ("client_id", p.client_id.as_str()),
                ("redirect_uri", redirect_uri.as_str()),
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
            .config
            .oauth_provider
            .get(&s.provider)
            .ok_or(Error::NotFound)?;
        let redirect_uri: Url = self
            .state
            .config
            .api_url
            .join(&format!("/api/v1/auth/oauth/{}/redirect", s.provider))?;
        let body = OauthTokenExchange {
            grant_type: "authorization_code".to_string(),
            code,
            redirect_uri: redirect_uri.into(),
        };

        let res: OauthTokenResponse = client
            .post(&p.token_url)
            .basic_auth(&p.client_id, Some(&p.client_secret))
            .header("Accept", "application/json")
            .header("User-Agent", &self.state.config.url_preview.user_agent)
            .form(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok((res, s.session_id))
    }

    pub async fn revoke_token(&self, provider: &str, token: String) -> Result<()> {
        let p = self
            .state
            .config
            .oauth_provider
            .get(provider)
            .ok_or(Error::NotFound)?;
        let client = reqwest::Client::new();
        let body = OauthTokenRevoke {
            token_type_hint: "access_token".to_string(),
            token,
        };
        client
            .post(&p.revocation_url)
            .basic_auth(&p.client_id, Some(&p.client_secret))
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
            .header("User-Agent", &self.state.config.url_preview.user_agent)
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
            .header("User-Agent", &self.state.config.url_preview.user_agent)
            .bearer_auth(token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(res)
    }
}
