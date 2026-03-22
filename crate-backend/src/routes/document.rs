// Document routes - stub migration
// Note: Most endpoints return Unimplemented due to missing data layer methods

use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use common::v1::routes;
use common::v1::types::application::Scope;
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;

use crate::error::{Error, Result};
use crate::routes::util::Auth;
use crate::{routes2, ServerState};

/// Wiki history
#[handler(routes::wiki_history)]
async fn wiki_history(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::wiki_history::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Document branch list
#[handler(routes::document_branch_list)]
async fn document_branch_list(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::document_branch_list::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Document branch get
#[handler(routes::document_branch_get)]
async fn document_branch_get(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::document_branch_get::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Document branch create
#[handler(routes::document_branch_create)]
async fn document_branch_create(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::document_branch_create::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Document branch patch
#[handler(routes::document_branch_patch)]
async fn document_branch_patch(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::document_branch_patch::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Document branch delete
#[handler(routes::document_branch_delete)]
async fn document_branch_delete(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::document_branch_delete::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Document branch merge
#[handler(routes::document_branch_merge)]
async fn document_branch_merge(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::document_branch_merge::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Document CRDT diff
#[handler(routes::document_crdt_diff)]
async fn document_crdt_diff(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::document_crdt_diff::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Document tag create
#[handler(routes::document_tag_create)]
async fn document_tag_create(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::document_tag_create::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Document tag patch
#[handler(routes::document_tag_patch)]
async fn document_tag_patch(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::document_tag_patch::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Document tag delete
#[handler(routes::document_tag_delete)]
async fn document_tag_delete(
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::document_tag_delete::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(wiki_history))
        .routes(routes2!(document_branch_list))
        .routes(routes2!(document_branch_get))
        .routes(routes2!(document_branch_create))
        .routes(routes2!(document_branch_patch))
        .routes(routes2!(document_branch_delete))
        .routes(routes2!(document_branch_merge))
        .routes(routes2!(document_crdt_diff))
        .routes(routes2!(document_tag_create))
        .routes(routes2!(document_tag_patch))
        .routes(routes2!(document_tag_delete))
}
