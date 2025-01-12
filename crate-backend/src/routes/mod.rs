use utoipa_axum::router::OpenApiRouter;

use crate::ServerState;

mod util;
mod media;
mod room;
mod thread;
mod message;

pub fn routes() -> OpenApiRouter<ServerState> {
    OpenApiRouter::new()
        .merge(room::routes())
        .merge(media::routes())
        .merge(thread::routes())
        .merge(message::routes())
}
