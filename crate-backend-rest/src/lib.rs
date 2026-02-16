use lamprey_backend_core::prelude::*;

pub mod debug;

pub async fn router() -> axum::Router {
    axum::Router::new()
        .merge(debug::routes())
}
