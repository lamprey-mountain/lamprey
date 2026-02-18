//! miscellaneous types

use std::fmt::Display;

#[cfg(feature = "serde")]
use serde::Deserialize;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

pub mod color;
pub mod time;
pub use color::{Color, ColorSemantic, ColorThemed};
pub use time::Time;

use super::{ApplicationId, SessionId, UserId};

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum UserIdReq {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "const_self"))]
    UserSelf,
    UserId(UserId),
}

#[derive(Debug, Deserialize, ToSchema)]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum ApplicationIdReq {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "const_self"))]
    AppSelf,
    ApplicationId(ApplicationId),
}

#[derive(Debug, Deserialize, ToSchema)]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum SessionIdReq {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "const_self"))]
    SessionSelf,
    SessionId(SessionId),
}

// TODO: deserialize as @host and @client
#[derive(Debug, Deserialize, ToSchema)]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum ServerReq {
    /// the target server
    #[cfg_attr(feature = "serde", serde(deserialize_with = "const_host"))]
    ServerHost,

    /// the requesting server
    ///
    /// intended to be used with federation. non-server clients cannot use this.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "const_client"))]
    ServerClient,

    /// references a server by its fully qualified domain name
    // rename to ServerHostname?
    ServerFqdn(String),
}

fn const_self<'de, D>(deserializer: D) -> std::result::Result<(), D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    enum Helper {
        #[cfg_attr(feature = "serde", serde(rename = "@self"))]
        Variant,
    }

    Helper::deserialize(deserializer).map(|_| ())
}

fn const_host<'de, D>(deserializer: D) -> std::result::Result<(), D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    enum Helper {
        #[cfg_attr(feature = "serde", serde(rename = "@host"))]
        Variant,
    }

    Helper::deserialize(deserializer).map(|_| ())
}

fn const_client<'de, D>(deserializer: D) -> std::result::Result<(), D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    enum Helper {
        #[cfg_attr(feature = "serde", serde(rename = "@client"))]
        Variant,
    }

    Helper::deserialize(deserializer).map(|_| ())
}

// fn const_all<'de, D>(deserializer: D) -> std::result::Result<(), D::Error>
// where
//     D: serde::Deserializer<'de>,
// {
//     #[derive(Deserialize)]
//     enum Helper {
//         #[cfg_attr(feature = "serde", serde(rename = "@all"))]
//         Variant,
//     }

//     Helper::deserialize(deserializer).map(|_| ())
// }

impl UserIdReq {
    /// retrieve the user id, falling back to self_id if this is UserSelf
    // TODO: use this instead of manually matching
    // TODO: impl this for other FooIdReq types
    pub fn unwrap_or(self, self_id: UserId) -> UserId {
        match self {
            UserIdReq::UserSelf => self_id,
            UserIdReq::UserId(user_id) => user_id,
        }
    }
}

impl Display for UserIdReq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserIdReq::UserSelf => write!(f, "@self"),
            UserIdReq::UserId(user_id) => write!(f, "{user_id}"),
        }
    }
}

impl Display for ApplicationIdReq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApplicationIdReq::AppSelf => write!(f, "@self"),
            ApplicationIdReq::ApplicationId(app_id) => write!(f, "{app_id}"),
        }
    }
}

impl Display for ServerReq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerReq::ServerHost => write!(f, "@host"),
            ServerReq::ServerClient => write!(f, "@client"),
            ServerReq::ServerFqdn(fqdn) => write!(f, "{fqdn}"),
        }
    }
}
