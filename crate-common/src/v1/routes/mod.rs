use bytes::Bytes;
use serde::de::DeserializeOwned;

// export all routes

// TODO: extract response(body = ...) from struct Response?

pub mod ack;
pub mod admin;
pub mod application;
pub mod auth;
pub mod automod;
pub mod calendar;
pub mod channel;
pub mod dm;
pub mod document;
pub mod e2ee;
pub mod emoji;
pub mod federation;
pub mod flume;
pub mod harvest;
pub mod interaction;
pub mod invite;
pub mod media;
pub mod message;
pub mod mirror;
pub mod moderation;
pub mod notification;
pub mod oauth;
pub mod pack;
pub mod permission_overwrite;
pub mod preferences;
pub mod push;
pub mod reaction;
pub mod redex;
pub mod relationship;
pub mod role;
pub mod room;
pub mod room_analytics;
pub mod room_member;
pub mod room_template;
pub mod search;
pub mod server;
pub mod session;
pub mod tag;
pub mod thread;
pub mod user;
pub mod user_connection;
pub mod user_email;
pub mod voice;
pub mod webhook;

/// route definitions for the cdn/media proxy
pub mod media_proxy;

pub use ack::*;
pub use application::*;
pub use auth::*;
pub use automod::*;
pub use calendar::*;
pub use channel::*;
pub use dm::*;
pub use document::*;
pub use emoji::*;
pub use federation::*;
pub use interaction::*;
pub use invite::*;
pub use media::*;
pub use message::*;
pub use mirror::*;
pub use moderation::*;
pub use notification::*;
pub use oauth::*;
pub use permission_overwrite::*;
pub use preferences::*;
pub use push::*;
pub use reaction::*;
pub use redex::*;
pub use relationship::*;
pub use role::*;
pub use room::*;
pub use room_analytics::*;
pub use room_member::*;
pub use room_template::*;
pub use search::*;
pub use server::*;
pub use session::*;
pub use tag::*;
pub use thread::*;
pub use user::*;
pub use user_connection::*;
pub use user_email::*;
pub use voice::*;
pub use webhook::*;

mod path_param;

pub use path_param::{PathParam, PathParamError};

/// Create an error response for invalid path matches
// TODO: better error response
pub fn invalid_path_error() -> http::Response<bytes::Bytes> {
    http::Response::builder()
        .status(http::StatusCode::NOT_FOUND)
        .body(bytes::Bytes::from("invalid path"))
        .unwrap()
}

// TEMP: compat
pub use crate::util::routes::Metadata as Endpoint;
pub use crate::util::routes::Method as EndpointMethod;

impl From<EndpointMethod> for ::utoipa::openapi::HttpMethod {
    fn from(m: EndpointMethod) -> Self {
        match m {
            EndpointMethod::Get => ::utoipa::openapi::HttpMethod::Get,
            EndpointMethod::Post => ::utoipa::openapi::HttpMethod::Post,
            EndpointMethod::Put => ::utoipa::openapi::HttpMethod::Put,
            EndpointMethod::Patch => ::utoipa::openapi::HttpMethod::Patch,
            EndpointMethod::Delete => ::utoipa::openapi::HttpMethod::Delete,
            EndpointMethod::Head => ::utoipa::openapi::HttpMethod::Head,
        }
    }
}

/// can extract body separately then extract with explicitly deserialized body later
pub trait ExtractableRequest: Sized {
    /// the request body
    type Body: DeserializeOwned;

    /// extract full request from parts and deserialized Body
    fn extract(
        parts: http::request::Parts,
        body: Self::Body,
    ) -> Result<Self, http::Response<Bytes>>;
}
