//! miscellaneous types

use std::{fmt::Display, str::FromStr};

#[cfg(feature = "serde")]
use serde::Deserialize;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

pub mod binary;
pub mod color;
pub mod metadata;
pub mod time;

pub use color::Color;
pub use time::Time;

use super::error::ApiError;
use super::{ApplicationId, SessionId, UserId};
use crate::util::is_valid_hostname;
use crate::v1::routes::{PathParam, PathParamError};
use crate::v1::types::error::{ApiResult, ErrorCode};
use crate::v1::types::federation::Hostname;
use crate::v1::types::{MediaId, MessageId};

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize), serde(untagged))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum UserIdReq {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "const_self"))]
    UserSelf,

    // TODO: rename to UserRemote
    RemoteUser(UserId, Hostname),

    UserId(UserId),
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize), serde(untagged))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum MediaIdReq {
    MediaRemote(MediaId, Hostname),
    MediaId(MediaId),
}

#[derive(Debug, Deserialize, ToSchema)]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum ApplicationIdReq {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "const_self"))]
    AppSelf,
    ApplicationId(ApplicationId),
}

#[derive(Debug)]
#[cfg_attr(
    feature = "serde",
    derive(Deserialize, serde::Serialize),
    serde(untagged)
)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum SessionIdReq {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "const_self"))]
    SessionSelf,
    SessionId(SessionId),
}

impl Display for SessionIdReq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionIdReq::SessionSelf => write!(f, "@self"),
            SessionIdReq::SessionId(id) => write!(f, "{}", id),
        }
    }
}

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
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_server_name"))]
    ServerName(String),
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize), serde(untagged))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum InteractionMessageReq {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "const_original"))]
    MessageOriginal,

    MessageId(MessageId),
}

#[cfg(feature = "serde")]
fn deserialize_server_name<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if is_valid_hostname(&s) {
        Ok(s)
    } else {
        Err(serde::de::Error::custom(format!("invalid hostname: {}", s)))
    }
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

fn const_original<'de, D>(deserializer: D) -> std::result::Result<(), D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    enum Helper {
        #[cfg_attr(feature = "serde", serde(rename = "@original"))]
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
    // TEMP: prevent warning spam
    // #[deprecated = "use local_unwrap_or"]
    pub fn unwrap_or(self, self_id: UserId) -> UserId {
        match self {
            UserIdReq::UserSelf => self_id,
            UserIdReq::RemoteUser(user_id, _) => user_id,
            UserIdReq::UserId(user_id) => user_id,
        }
    }

    /// retrieve the user id, rejecting remote users references, and falling back to self_id for UserSelf
    pub fn local_unwrap_or(self, self_id: UserId) -> ApiResult<UserId> {
        match self {
            UserIdReq::UserSelf => Ok(self_id),
            UserIdReq::RemoteUser(_, _) => {
                Err(ApiError::from_code(ErrorCode::CannotManageRemoteUser))
            }
            UserIdReq::UserId(user_id) => Ok(user_id),
        }
    }
}

impl Display for UserIdReq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserIdReq::UserSelf => write!(f, "@self"),
            UserIdReq::RemoteUser(user_id, host) => write!(f, "{user_id}:{host}"),
            UserIdReq::UserId(user_id) => write!(f, "{user_id}"),
        }
    }
}

impl PathParam for UserIdReq {
    fn from_path_param(s: &str) -> Result<Self, PathParamError> {
        if s == "@self" {
            Ok(UserIdReq::UserSelf)
        } else if let Some((uuid_str, host_str)) = s.split_once(':') {
            let user_id = UserId::from_str(uuid_str).map_err(|_| {
                PathParamError(format!("invalid remote user id uuid: {}", uuid_str))
            })?;
            if !is_valid_hostname(host_str) {
                return Err(PathParamError(format!(
                    "invalid hostname in remote user id: {}",
                    host_str
                )));
            }
            Ok(UserIdReq::RemoteUser(
                user_id,
                Hostname(host_str.to_string()),
            ))
        } else {
            UserId::from_str(s)
                .map(UserIdReq::UserId)
                .map_err(|_| PathParamError(format!("invalid user id: {}", s)))
        }
    }
}

impl PathParam for ApplicationIdReq {
    fn from_path_param(s: &str) -> Result<Self, PathParamError> {
        if s == "@self" {
            Ok(ApplicationIdReq::AppSelf)
        } else {
            ApplicationId::from_str(s)
                .map(ApplicationIdReq::ApplicationId)
                .map_err(|_| PathParamError(format!("invalid application id: {}", s)))
        }
    }
}

impl PathParam for SessionIdReq {
    fn from_path_param(s: &str) -> Result<Self, PathParamError> {
        if s == "@self" {
            Ok(SessionIdReq::SessionSelf)
        } else {
            SessionId::from_str(s)
                .map(SessionIdReq::SessionId)
                .map_err(|_| PathParamError(format!("invalid session id: {}", s)))
        }
    }
}

impl PathParam for ServerReq {
    fn from_path_param(s: &str) -> Result<Self, PathParamError> {
        if s == "@host" {
            Ok(ServerReq::ServerHost)
        } else if s == "@client" {
            Ok(ServerReq::ServerClient)
        } else if is_valid_hostname(s) {
            Ok(ServerReq::ServerName(s.to_owned()))
        } else {
            Err(PathParamError(format!("invalid hostname: {}", s)))
        }
    }
}

impl Display for InteractionMessageReq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InteractionMessageReq::MessageOriginal => write!(f, "@original"),
            InteractionMessageReq::MessageId(id) => write!(f, "{id}"),
        }
    }
}

impl PathParam for InteractionMessageReq {
    fn from_path_param(s: &str) -> Result<Self, PathParamError> {
        if s == "@original" {
            Ok(InteractionMessageReq::MessageOriginal)
        } else {
            MessageId::from_str(s)
                .map(InteractionMessageReq::MessageId)
                .map_err(|_| PathParamError(format!("invalid message id: {}", s)))
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
            ServerReq::ServerName(fqdn) => write!(f, "{fqdn}"),
        }
    }
}

// TODO: add a utility to serialize bytes as either unpadded urlsafe base64 (json) or raw binary (msgpack)
// struct Binary(Vec<u8>);
