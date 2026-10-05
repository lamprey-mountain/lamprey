use crate::prelude::*;
use routes::media_proxy as routes;

#[handler(routes::emoji_get)]
async fn get(_req: Req<routes::emoji_get::Endpoint>) -> Result<routes::emoji_get::Response> {
    todo!()
}

export_routes!(get);
