use axum::Router;
use utoipa_axum::router::OpenApiRouter;

use crate::ServerState;

pub fn routes() -> OpenApiRouter<ServerState> {
    OpenApiRouter::new()
        // .route("/room/{id}", get(room_get))
        // .route("/room", post(room_create))
}
