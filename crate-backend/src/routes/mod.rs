use std::sync::Arc;

use utoipa_axum::router::OpenApiRouter;

use crate::ServerState;

mod message;
mod room;
mod thread;
mod util;
// mod invite;
// mod role;
mod media;
// mod member;
mod auth;
mod search;
mod session;
mod sync;
mod user;

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .merge(room::routes())
        .merge(thread::routes())
        .merge(message::routes())
        // .merge(invite::routes())
        // .merge(role::routes())
        .merge(media::routes())
        // .merge(member::routes())
        .merge(sync::routes())
        .merge(user::routes())
        .merge(session::routes())
        .merge(auth::routes())
        .merge(search::routes())
}
