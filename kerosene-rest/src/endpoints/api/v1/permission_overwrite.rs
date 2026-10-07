use crate::{prelude::*, util::request::ReqAuth};
use common::v1::{routes, types::oauth::Scope};
use http::StatusCode;

#[handler(routes::permission_set)]
pub async fn set(
    req: Req<routes::permission_set::Endpoint>,
) -> Result<routes::permission_set::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Full])?;
    identity.ensure_user()?.ensure_unsuspended()?;

    let srv = req.services();
    let body = req.inner();

    let mut auth = ReqAuth::new(&req);
    srv.channels
        .permission_set(
            &mut auth,
            body.channel_id,
            body.overwrite_id,
            &body.overwrite,
        )
        .await?;

    Ok(routes::permission_set::Response {})
}

#[handler(routes::permission_remove)]
pub async fn remove(
    req: Req<routes::permission_remove::Endpoint>,
) -> Result<routes::permission_remove::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Full])?;
    identity.ensure_user()?.ensure_unsuspended()?;

    let srv = req.services();
    let body = req.inner();

    let mut auth = ReqAuth::new(&req);
    srv.channels
        .permission_delete(&mut auth, body.channel_id, body.overwrite_id)
        .await?;

    Ok(routes::permission_remove::Response {})
}

export_routes!(set, remove);
