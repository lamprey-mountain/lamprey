use crate::prelude::*;

use http::StatusCode;
use routes::media_proxy as routes;

#[handler(routes::media_head)]
async fn head(req: Req<routes::media_head::Endpoint>) -> Result<routes::media_head::Response> {
    req.inner().media_id;
    req.inner().query.wait;
    // TODO

    Ok(routes::media_head::Response {
        status: StatusCode::NOT_IMPLEMENTED,
    })
}

#[handler(routes::media_get)]
async fn get(req: Req<routes::media_get::Endpoint>) -> Result<routes::media_get::Response> {
    // TODO

    Ok(routes::media_get::Response {
        // ???
    })
}

export_routes!(head, get);
