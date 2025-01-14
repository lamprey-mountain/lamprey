use utoipa_axum::router::OpenApiRouter;

use crate::ServerState;

mod util;
mod room;
mod thread;
mod message;
// mod invite;
// mod role;
mod media;
// mod member;
mod sync;
// mod user;
// mod session;
// mod auth;

pub fn routes() -> OpenApiRouter<ServerState> {
    OpenApiRouter::new()
        .merge(room::routes())
        .merge(thread::routes())
        .merge(message::routes())
        // .merge(invite::routes())
        // .merge(role::routes())
        .merge(media::routes())
        // .merge(member::routes())
        .merge(sync::routes())
        // .merge(user::routes())
        // .merge(session::routes())
        // .merge(auth::routes())
}
