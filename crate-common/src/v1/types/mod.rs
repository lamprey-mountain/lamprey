// TEMP: suppress warnings before i remove everything
// #![allow(deprecated)]

pub mod admin;
pub mod application;
pub mod audit_logs;
pub mod auth;
pub mod automod;
pub mod calendar;
pub mod channel;
pub mod document;
pub mod email;
pub mod embed;
pub mod emoji;
pub mod error;
pub mod harvest;
pub mod ids;
pub mod invite;
pub mod media;
pub mod message;
pub mod misc;
pub mod moderation;
pub mod notifications;
pub mod oauth;
pub mod pagination;
pub mod permission;
pub mod presence;
pub mod push;
pub mod reaction;
pub mod role;
pub mod room;
pub mod room_analytics;
pub mod room_member;
pub mod room_template;
pub mod search;
pub mod server;
pub mod session;
pub mod sync;
pub mod tag;
pub mod text;
pub mod thread_member;
pub mod user;
pub mod user_config;
pub mod util;
pub mod visibility;
pub mod voice;
pub mod webhook;

#[cfg(feature = "feat_interaction")]
pub mod interactions;

pub use media::{
    Audio, Image, Media, MediaCreate, MediaCreateSource, MediaPatch, MediaTrack, MediaTrackInfo,
    Mime, Mixed, Text, TimedText, TrackSource, Video,
};

// TODO: probably should stop exporting *everything*
pub use audit_logs::*;
pub use channel::*;
pub use embed::*;
pub use ids::*;
pub use invite::*;
pub use message::*;
pub use pagination::*;
pub use permission::*;
pub use role::*;
pub use room::*;
pub use room_member::*;
pub use session::*;
pub use sync::*;
pub use thread_member::*;
pub use user::*;
