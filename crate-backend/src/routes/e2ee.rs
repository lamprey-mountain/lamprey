use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use common::v1::routes;
use common::v1::types::application::Scope;
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;

use crate::prelude::*;
use crate::routes::util::auth::Auth4;
use crate::routes2;

#[handler(routes::e2ee_upload_cross_signing_keys)]
async fn upload_csk(
    auth: Auth4,
    State(globals): State<Globals>,
    req: routes::e2ee_upload_cross_signing_keys::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = globals.services();
    // srv.e2ee.upload_csk(req).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[handler(routes::e2ee_upload_session_key)]
async fn upload_session(
    auth: Auth4,
    State(globals): State<Globals>,
    req: routes::e2ee_upload_session_key::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = globals.services();
    // srv.e2ee.upload_session(req).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[handler(routes::e2ee_upload_mls_key_packages)]
async fn upload_mls(
    auth: Auth4,
    State(globals): State<Globals>,
    req: routes::e2ee_upload_mls_key_packages::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = globals.services();
    // srv.e2ee.upload_mls(req).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[handler(routes::e2ee_upload_cross_signing_signatures)]
async fn upload_csk_signatures(
    auth: Auth4,
    State(globals): State<Globals>,
    req: routes::e2ee_upload_cross_signing_signatures::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = globals.services();
    // srv.e2ee.upload_csk_signatures(req).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[handler(routes::e2ee_query_cross_signing_keys)]
async fn query_csk(
    auth: Auth4,
    State(globals): State<Globals>,
    req: routes::e2ee_query_cross_signing_keys::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = globals.services();
    // let res = srv.e2ee.query_csk(req).await?;
    Ok(Json(res))
}

#[handler(routes::e2ee_claim_mls_key_packages)]
async fn claim_mls(
    auth: Auth4,
    State(globals): State<Globals>,
    req: routes::e2ee_claim_mls_key_packages::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = globals.services();
    // let res = srv.e2ee.claim_mls(req).await?;
    Ok(Json(res))
}

#[handler(routes::e2ee_mls_welcome)]
async fn mls_welcome(
    auth: Auth4,
    State(globals): State<Globals>,
    req: routes::e2ee_mls_welcome::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = globals.services();
    // srv.e2ee.mls_welcome(req).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[handler(routes::e2ee_mls_commit)]
async fn mls_commit(
    auth: Auth4,
    State(globals): State<Globals>,
    req: routes::e2ee_mls_commit::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = globals.services();
    // srv.e2ee.mls_commit(req).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[handler(routes::e2ee_mls_message)]
async fn mls_message(
    auth: Auth4,
    State(globals): State<Globals>,
    req: routes::e2ee_mls_message::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = globals.services();
    // srv.e2ee.mls_message(req).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[handler(routes::e2ee_mls_epoch)]
async fn mls_epoch(
    auth: Auth4,
    State(globals): State<Globals>,
    req: routes::e2ee_mls_epoch::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = globals.services();
    // srv.e2ee.mls_epoch(req).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[handler(routes::e2ee_keyshare_request)]
async fn keyshare_request(
    auth: Auth4,
    State(globals): State<Globals>,
    req: routes::e2ee_keyshare_request::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = globals.services();
    // srv.e2ee.keyshare_request(req).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[handler(routes::e2ee_keyshare_respond)]
async fn keyshare_respond(
    auth: Auth4,
    State(globals): State<Globals>,
    req: routes::e2ee_keyshare_respond::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = globals.services();
    // srv.e2ee.keyshare_respond(req).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> OpenApiRouter<Globals> {
    OpenApiRouter::new()
        .routes(routes2!(upload_csk))
        .routes(routes2!(upload_session))
        .routes(routes2!(upload_mls))
        .routes(routes2!(upload_csk_signatures))
        .routes(routes2!(query_csk))
        .routes(routes2!(claim_mls))
        .routes(routes2!(mls_welcome))
        .routes(routes2!(mls_commit))
        .routes(routes2!(mls_message))
        .routes(routes2!(mls_epoch))
        .routes(routes2!(keyshare_request))
        .routes(routes2!(keyshare_respond))
}
