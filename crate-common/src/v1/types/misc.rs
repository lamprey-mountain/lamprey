//! miscellaneous types

use std::fmt::Display;

use serde::Deserialize;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

pub mod color;
pub mod time;
pub use color::{Color, ColorSemantic, ColorThemed};
pub use time::Time;

use super::{ApplicationId, SessionId, UserId};

#[derive(Debug, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum UserIdReq {
    #[serde(deserialize_with = "const_self")]
    UserSelf,
    UserId(UserId),
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum ApplicationIdReq {
    #[serde(deserialize_with = "const_self")]
    AppSelf,
    ApplicationId(ApplicationId),
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum SessionIdReq {
    #[serde(deserialize_with = "const_self")]
    SessionSelf,
    // #[serde(deserialize_with = "const_all")]
    // SessionAll,
    SessionId(SessionId),
}

fn const_self<'de, D>(deserializer: D) -> std::result::Result<(), D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    enum Helper {
        #[serde(rename = "@self")]
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
//         #[serde(rename = "@all")]
//         Variant,
//     }

//     Helper::deserialize(deserializer).map(|_| ())
// }

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
