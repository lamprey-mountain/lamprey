use std::sync::Arc;

use utoipa_axum::router::OpenApiRouter;

use crate::ServerState;

mod auth;
mod debug;
mod dm;
mod emoji;
mod invite;
mod media;
mod message;
mod moderation;
mod notifications;
mod permission_overwrite;
mod reaction;
mod relationship;
mod role;
mod room;
mod room_member;
mod search;
mod session;
mod sync;
mod tag;
mod thread;
mod thread_member;
mod user;
mod user_config;
mod user_email;
mod util;

// HACK: re-export because utoipa
pub use user::UserListFilter;

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .merge(auth::routes())
        .merge(debug::routes())
        .merge(dm::routes())
        .merge(emoji::routes())
        .merge(invite::routes())
        .merge(media::routes())
        .merge(message::routes())
        .merge(moderation::routes())
        .merge(notifications::routes())
        .merge(permission_overwrite::routes())
        .merge(reaction::routes())
        .merge(relationship::routes())
        .merge(role::routes())
        .merge(room::routes())
        .merge(room_member::routes())
        .merge(search::routes())
        .merge(session::routes())
        .merge(sync::routes())
        .merge(tag::routes())
        .merge(thread::routes())
        .merge(thread_member::routes())
        .merge(user::routes())
        .merge(user_config::routes())
        .merge(user_email::routes())
}
