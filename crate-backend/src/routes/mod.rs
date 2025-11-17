use std::sync::Arc;

use utoipa_axum::router::OpenApiRouter;

use crate::ServerState;

mod admin;
mod application;
mod auth;
mod automod;
mod calendar;
mod channel;
mod debug;
mod dm;
mod emoji;
mod internal;
mod invite;
mod media;
mod message;
mod moderation;
mod notification;
mod permission_overwrite;
mod public;
mod reaction;
mod relationship;
mod role;
mod room;
mod room_member;
mod room_template;
mod search;
mod session;
mod sync;
mod tag;
mod thread;
mod user;
mod user_config;
mod user_email;
mod util;
mod voice;
mod webhook;

pub mod metrics;

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .merge(admin::routes())
        .merge(application::routes())
        .merge(auth::routes())
        .merge(automod::routes())
        .merge(calendar::routes())
        .merge(debug::routes())
        .merge(dm::routes())
        .merge(emoji::routes())
        .merge(internal::routes())
        .merge(invite::routes())
        .merge(media::routes())
        .merge(message::routes())
        .merge(moderation::routes())
        .merge(notification::routes())
        .merge(permission_overwrite::routes())
        .merge(public::routes())
        .merge(reaction::routes())
        .merge(relationship::routes())
        .merge(role::routes())
        .merge(room::routes())
        .merge(room_member::routes())
        .merge(room_template::routes())
        .merge(search::routes())
        .merge(session::routes())
        .merge(sync::routes())
        .merge(channel::routes())
        .merge(tag::routes())
        .merge(thread::routes())
        .merge(user::routes())
        .merge(user_config::routes())
        .merge(user_email::routes())
        .merge(voice::routes())
        .merge(webhook::routes())
}
