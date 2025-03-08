// TEMP: suppress warnings before i remove everything
#![allow(deprecated)]

// TODO(#242): make serde optional
// maybe merge util and misc?

/// utility stuff
pub mod util;

/// miscellaneous types
pub mod misc;

pub mod audit_logs;
pub mod auth;
pub mod email;
pub mod embed;
pub mod emoji;
pub mod ids;
pub mod invite;
pub mod media;
pub mod message;
pub mod moderation;
pub mod notifications;
pub mod pagination;
pub mod permission;
pub mod profile;

#[cfg(feature = "feat_reactions")]
pub mod reaction;

// pub mod redex;
pub mod role;
pub mod room;
pub mod room_member;
pub mod search;
pub mod session;
pub mod sync;
pub mod tag;
pub mod text;
pub mod thread;
pub mod thread_member;
pub mod user;
pub mod user_config;
pub mod user_status;
pub mod voice;

pub use media::{
    Audio, Image, Media, MediaCreate, MediaCreateSource, MediaPatch, MediaSize, MediaTrack,
    MediaTrackInfo, Mime, Mixed, Text, TimedText, TrackSource, Video,
};

// TODO: probably should stop exporting *everything*
pub use audit_logs::*;
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
pub use thread::*;
pub use thread_member::*;
pub use user::*;
