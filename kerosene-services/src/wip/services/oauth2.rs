use std::time::Instant;

use common::{v1::types::oauth::OauthTokenResponse, v2::types::SessionId};
use url::Url;
use uuid::Uuid;

use crate::prelude::*;

pub struct Service {
    globals: Globals,
}

impl Service {
    pub fn new(_globals: Globals) -> Self {
        todo!()
    }

    /// build a url clients should redirect to
    pub fn create_url(&self, _provider: &str, _session_id: SessionId) -> Result<Url> {
        todo!()
    }

    /// handle a token exchange request
    pub async fn exchange_code_for_token(
        &self,
        _state: Uuid,
        _code: String,
    ) -> Result<(OauthTokenResponse, SessionId)> {
        todo!()
    }

    /// revoke a provider's oauth token
    pub async fn revoke_token(&self, _provider: &str, _token: String) -> Result<()> {
        todo!()
    }

    // TODO: background job to refresh expiring oauth tokens?
}

pub struct OauthState {
    provider: String,
    session_id: SessionId,
    created_at: Instant,
}

// TODO: import profile picture
#[cfg(any())]
pub trait Oauth2Provider {
    async fn fetch_profile(&self, token: &str) -> Result<Profile>;
}

// pub enum OauthProviderKind {
//     Discord,
//     Github,
//     OpenId,
// }

#[cfg(any())]
mod discord {
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
}

#[cfg(any())]
mod github {
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
}
