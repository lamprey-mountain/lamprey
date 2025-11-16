use utoipa_axum::router::OpenApiRouter;

use crate::AppState;

mod emoji;
mod gifv;
mod media;
mod thumb;
mod util;

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .merge(media::routes())
        .merge(thumb::routes())
        .merge(emoji::routes())
        .merge(gifv::routes())
}
