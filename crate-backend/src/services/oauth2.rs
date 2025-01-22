// TODO: port to https://docs.rs/oauth2/latest/oauth2/
// TODO: make more generic

use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use super::Services;
use crate::error::Result;

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
    // /// the current application
    // application: DiscordApplication,

    // /// the scopes the user has authorized the application for
    // scopes: Vec<String>,

    // /// ISO8601 timestamp when the access token expires
    // expires: String,
    /// the user who has authorized, if the user has authorized with the identify scope
    ///
    /// i'm assuming that `user` always exists for now
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
    // avatar	?string	the user's avatar hash	identify
    // bot?	boolean	whether the user belongs to an OAuth2 application	identify
    // system?	boolean	whether the user is an Official Discord System user (part of the urgent message system)	identify
    // mfa_enabled?	boolean	whether the user has two factor enabled on their account	identify
    // banner?	?string	the user's banner hash	identify
    // accent_color?	?integer	the user's banner color encoded as an integer representation of hexadecimal color code	identify
    // locale?	string	the user's chosen language option	identify
    // verified?	boolean	whether the email on this account has been verified	email
    // email?	?string	the user's email	email
    // flags?	integer	the flags on a user's account	identify
    // premium_type?	integer	the type of Nitro subscription on a user's account	identify
    // public_flags?	integer	the public flags on a user's account	identify
    // avatar_decoration_data?	?avatar decoration data object	data for the user's avatar decoration	identify
}

impl Services {
    pub fn oauth_create_url(&self) -> Result<String> {
        let state = Uuid::new_v4();
        self.state.valid_oauth2_states.insert(state);
        let dc = &self.state.config.discord;
        let url = Url::parse_with_params(
            "https://canary.discord.com/oauth2/authorize",
            [
                ("client_id", dc.client_id.as_str()),
                ("response_type", "code"),
                ("redirect_uri", &dc.redirect_uri),
                ("scope", "identify"),
                ("state", &state.to_string()),
            ],
        )
        .expect("invalid url?");
        Ok(url.to_string())
    }

    pub async fn oauth_exchange_code_for_token(&self, code: String) -> Result<OauthTokenResponse> {
        let client = reqwest::Client::new();

        let dc = &self.state.config.discord;
        let body = OauthTokenExchange {
            grant_type: "authorization_code".to_string(),
            code,
            redirect_uri: dc.redirect_uri.clone(),
        };

        let res: OauthTokenResponse = client
            .post("https://discord.com/api/v10/oauth2/token")
            .basic_auth(&dc.client_id, Some(&dc.client_secret))
            .form(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(res)
    }

    pub async fn oauth_get_user(&self, token: String) -> Result<DiscordAuth> {
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

    pub async fn oauth_revoke_token(&self, token: String) -> Result<()> {
        let client = reqwest::Client::new();
        let body = OauthTokenRevoke {
            token_type_hint: "access_token".to_string(),
            token,
        };
        let dc = &self.state.config.discord;
        client
            .post("https://discord.com/api/v10/oauth2/token/revoke")
            .basic_auth(&dc.client_id, Some(&dc.client_secret))
            .form(&body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}
