use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};
use common::v1::types::util::Time;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::error::Result;
use crate::{Error, ServerState};

// this is intended for role/permission edits, but is it really useful with
// if-match/etags? this wont help much with performance, mvcc would be a
// pain, seems kind of useless besides being able to succeed/fail a bunch of
// operations but even then thats not that useful
//
// just adding here for posterity, since this is an idea i've had a lot
/// A batch of operations, executed atomically
#[allow(unused)]
pub struct Batch {
    /// when this was created
    pub created_at: Time,

    /// when this is automatically rolled back (defaults to 60 seconds)
    pub expires_at: Time,

    /// max http requests for this batch (defaults to 100 operations)
    pub max_ops: u32,
}

/// Batch create (TODO)
#[utoipa::path(
    post,
    path = "/batch",
    tags = ["batch"],
    responses(
        (status = OK, body = (), description = "ok"),
    )
)]
async fn batch_create(State(_s): State<Arc<ServerState>>) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Batch commit (TODO)
#[utoipa::path(
    post,
    path = "/batch/{batch_id}/commit",
    params(
        ("batch_id" = String, Path, description = "Batch ID to commit")
    ),
    tags = ["batch"],
    responses(
        (status = 200, body = (), description = "Batch committed successfully"),
    )
)]
async fn batch_commit(
    State(_s): State<Arc<ServerState>>,
    Path(_batch_id): Path<String>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Batch rollback/delete (TODO)
#[utoipa::path(
    delete,
    path = "/batch/{batch_id}",
    params(
        ("batch_id" = String, Path, description = "Batch ID to rollback/delete")
    ),
    tags = ["batch"],
    responses(
        (status = 200, body = (), description = "Batch rolled back successfully"),
    )
)]
async fn batch_rollback(
    State(_s): State<Arc<ServerState>>,
    Path(_batch_id): Path<String>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(batch_create))
        .routes(routes!(batch_commit))
        .routes(routes!(batch_rollback))
}
