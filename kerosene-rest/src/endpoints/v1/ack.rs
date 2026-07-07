use crate::prelude::*;

#[handler(routes::ack_bulk)]
#[axum::debug_handler]
async fn bulk(req: Req<routes::ack_bulk::Endpoint>) -> Result<routes::ack_bulk::Response> {
    // req.auth.ensure_scopes(&[Scope::Full])?;
    // TODO

    Ok(routes::ack_bulk::Response {})
}
