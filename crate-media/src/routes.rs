use utoipa_axum::router::OpenApiRouter;

use crate::AppState;

mod emoji;
mod gifv;
mod media;
mod stream;
mod thumb;
mod trickplay;
mod util;

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .merge(emoji::routes())
        .merge(gifv::routes())
        .merge(media::routes())
        .merge(stream::routes())
        .merge(thumb::routes())
        .merge(trickplay::routes())
}
