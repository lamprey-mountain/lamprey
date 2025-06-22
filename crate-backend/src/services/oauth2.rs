// TODO: port to https://docs.rs/oauth2/latest/oauth2/
// TODO: make more generic

use std::sync::Arc;

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
    pub name: String,
}

pub struct OauthState {
    provider: String,
    session_id: SessionId,
}

impl OauthState {
    pub fn new(provider: String, session_id: SessionId) -> Self {
        Self {
            provider,
            session_id,
        }
    }
}

pub struct ServiceOauth {
    state: Arc<ServerStateInner>,
    oauth_states: Arc<DashMap<Uuid, OauthState>>,
}

impl ServiceOauth {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            oauth_states: Arc::new(DashMap::new()),
        }
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
        let (_, s) = self
            .oauth_states
            .remove(&state)
            .ok_or(Error::BadStatic("invalid or expired state"))?;
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
        // let res: serde_json::Value = client
        //     .get("https://api.github.com/user")
        //     .header("accept", "application/vnd.github+json")
        //     .header("X-GitHub-Api-Version", "2022-11-28")
        //     .bearer_auth(token)
        //     .send()
        //     .await?
        //     .error_for_status()?
        //     .json()
        //     .await?;
        // res.get("");
        let res: GithubUser = client
            .get("https://api.github.com/user")
            .header("accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .bearer_auth(token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(res)
    }
}
