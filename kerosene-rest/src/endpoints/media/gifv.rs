use crate::prelude::*;
use routes::media_proxy as routes;

#[handler(routes::gifv_get)]
async fn get(_req: Req<routes::gifv_get::Endpoint>) -> Result<routes::gifv_get::Response> {
    todo!()
}

export_routes!(get);
