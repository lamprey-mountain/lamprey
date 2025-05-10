use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use common::v1::types::{MessageId, MessageThreadUpdate, ThreadState, ThreadType};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::{
    types::{
        DbMessageCreate, DbThreadCreate, MessageSync, MessageType, MessageVerId, PaginationQuery,
        PaginationResponse, Permission, RoomId, Thread, ThreadCreate, ThreadId, ThreadPatch,
    },
    Error, ServerState,
};

use super::util::{Auth, HeaderReason};
use crate::error::Result;

/// Voice do something (TEMP)
#[utoipa::path(
    post,
    path = "/thread/{thread_id}/voice",
    params(("thread_id", description = "Thread id")),
    tags = ["voice"],
    responses(
        (status = CREATED, body = Thread, description = "Create thread success"),
    )
)]
async fn voice_foobar(
    Path((_room_id,)): Path<(RoomId,)>,
    Auth(_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    // HeaderReason(_reason): HeaderReason,
    // Json(_json): Json<ThreadCreate>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new().routes(routes!(voice_foobar))
}
