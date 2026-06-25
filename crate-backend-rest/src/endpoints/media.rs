use crate::prelude::*;

#[handler(routes::ack_bulk_new)]
async fn bulk(req: Req<routes::ack_bulk_new::Endpoint>) -> Result<routes::ack_bulk_new::Response> {
    // req.auth.ensure_scopes(&[Scope::Full])?;
    // TODO

    Ok(routes::ack_bulk_new::Response {})
}
