use crate::prelude::*;

#[handler(routes::auth_oauth_init)]
pub async fn oauth_init(
    req: Req<routes::auth_oauth_init::Endpoint>,
) -> Result<routes::auth_oauth_init::Response> {
    let session = req.identity().ensure_session()?;
    let url = req
        .services()
        .oauth
        .create_url(&req.inner().provider, session.id)
        .cast_internal()?;
    Ok(routes::auth_oauth_init::Response {
        oauth: routes::OauthInitResponse { url },
    })
}

export_routes!(oauth_init);
