use std::sync::Arc;

use utoipa_axum::router::OpenApiRouter;

use crate::ServerState;

mod auth;
mod invite;
mod media;
mod message;
mod role;
mod room;
mod room_member;
mod search;
mod session;
mod sync;
mod thread;
mod user;
mod util;

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .merge(room::routes())
        .merge(thread::routes())
        .merge(message::routes())
        .merge(invite::routes())
        .merge(role::routes())
        .merge(media::routes())
        .merge(room_member::routes())
        .merge(sync::routes())
        .merge(user::routes())
        .merge(session::routes())
        .merge(auth::routes())
        .merge(search::routes())
}
