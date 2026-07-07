use crate::prelude::*;
use routes::media_proxy as r;

#[handler(r::media_get)]
async fn media_get(req: Req<r::media_get::Endpoint>) -> Result<r::media_get::Response> {
    // req.auth.ensure_scopes(&[Scope::Full])?;
    // TODO

    Ok(r::media_get::Response {})
}
