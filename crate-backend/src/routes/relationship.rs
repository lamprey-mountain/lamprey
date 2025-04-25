use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::{
    MessageSync, PaginationQuery, PaginationResponse, Relationship, RelationshipPatch,
    RelationshipType, RelationshipWithUserId, UserId,
};
use http::StatusCode;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::types::UserIdReq;
use crate::ServerState;

use super::util::Auth;
use crate::error::{Error, Result};

/// Relationship get
///
/// Get your relationship with another user
#[utoipa::path(
    get,
    // path = "/user/@self/memo/{target_id}",
    // path = "/user/@self/note/{target_id}",
    path = "/user/@self/relationship/{target_id}",
    params(
        ("target_id", description = "Target user's id"),
    ),
    tags = ["relationship"],
    responses(
        (status = OK, body = Relationship, description = "success"),
        (status = NOT_FOUND, description = "couldn't find that user or you don't have any relationship state yet"),
    )
)]
#[deprecated = "get the target user directly"]
async fn relationship_get(
    Path(target_user_id): Path<UserId>,
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let rel = data
        .user_relationship_get(auth_user_id, target_user_id)
        .await?;
    if let Some(rel) = rel {
        Ok(Json(rel))
    } else {
        Err(Error::NotFound)
    }
}

/// Relationship update
///
/// Update your relationship with another user
// TEMP: ugly hacky code
#[utoipa::path(
    patch,
    path = "/user/@self/relationship/{target_id}",
    params(
        ("target_id", description = "Target user's id"),
    ),
    tags = ["relationship"],
    responses(
        (status = OK, body = Relationship, description = "success"),
        (status = NOT_MODIFIED, description = "not modified"),
    )
)]
#[deprecated = "will be split into different routes depending on relationship action"]
async fn relationship_update(
    Path(target_user_id): Path<UserId>,
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
    Json(patch): Json<RelationshipPatch>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let mut is_friend_req = false;
    let mut is_friend_accept = false;
    if let Some(rel_ty) = &patch.relation {
        let old_rel_ty = data
            .user_relationship_get(auth_user_id, target_user_id)
            .await?
            .and_then(|r| dbg!(r).relation);
        if &old_rel_ty != rel_ty {
            let rev_rel_ty = data
                .user_relationship_get(target_user_id, auth_user_id)
                .await?
                .and_then(|r| dbg!(r).relation);
            match rel_ty {
                Some(RelationshipType::Friend) => {
                    if old_rel_ty != Some(RelationshipType::Incoming)
                        || rev_rel_ty != Some(RelationshipType::Outgoing)
                    {
                        return Err(Error::BadStatic("need to send a friend request"));
                    }
                    is_friend_accept = true;
                }
                Some(RelationshipType::Incoming) => return Err(Error::BadStatic("cant do that")),
                Some(RelationshipType::Outgoing) => {
                    if rev_rel_ty == Some(RelationshipType::Block) {
                        return Err(Error::Blocked);
                    }
                    is_friend_req = true;
                }
                Some(RelationshipType::Block) | None => {}
            }

            assert_eq!(
                old_rel_ty == Some(RelationshipType::Incoming),
                rev_rel_ty == Some(RelationshipType::Outgoing)
            );

            if is_friend_req && (old_rel_ty == Some(RelationshipType::Incoming)) {
                return Err(Error::BadStatic("need to accept a friend request"));
            }
        }
    }
    data.user_relationship_edit(auth_user_id, target_user_id, patch)
        .await?;
    if is_friend_req {
        data.user_relationship_edit(
            target_user_id,
            auth_user_id,
            RelationshipPatch {
                note: None,
                petname: None,
                ignore: None,
                relation: Some(Some(RelationshipType::Incoming)),
            },
        )
        .await?;
        let rev_rel = data
            .user_relationship_get(target_user_id, auth_user_id)
            .await?
            .unwrap();
        s.broadcast(MessageSync::RelationshipUpsert {
            user_id: target_user_id,
            relationship: rev_rel,
        })?;
    } else if is_friend_accept {
        data.user_relationship_edit(
            target_user_id,
            auth_user_id,
            RelationshipPatch {
                note: None,
                petname: None,
                ignore: None,
                relation: Some(Some(RelationshipType::Friend)),
            },
        )
        .await?;
        let rev_rel = data
            .user_relationship_get(target_user_id, auth_user_id)
            .await?
            .unwrap();
        s.broadcast(MessageSync::RelationshipUpsert {
            user_id: target_user_id,
            relationship: rev_rel,
        })?;
    }
    let rel = data
        .user_relationship_get(auth_user_id, target_user_id)
        .await?
        .unwrap();
    s.broadcast(MessageSync::RelationshipUpsert {
        user_id: auth_user_id,
        relationship: rel.clone(),
    })?;
    Ok(Json(rel))
}

/// Relationship remove
///
/// Reset your relationship with another user
#[utoipa::path(
    delete,
    path = "/user/@self/relationship/{target_id}",
    params(
        ("target_id", description = "Target user's id"),
    ),
    tags = ["relationship"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
#[deprecated = "will be split into different routes depending on relationship action"]
async fn relationship_reset(
    Path(target_user_id): Path<UserId>,
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    data.user_relationship_delete(auth_user_id, target_user_id)
        .await?;
    let rev_rel = data
        .user_relationship_get(target_user_id, auth_user_id)
        .await?;
    if rev_rel.is_some_and(|r| r.relation != Some(RelationshipType::Block)) {
        data.user_relationship_edit(
            target_user_id,
            auth_user_id,
            RelationshipPatch {
                note: None,
                petname: None,
                ignore: None,
                relation: Some(None),
            },
        )
        .await?;
        let rev_rel = data
            .user_relationship_get(target_user_id, auth_user_id)
            .await?
            .unwrap();
        s.broadcast(MessageSync::RelationshipUpsert {
            user_id: target_user_id,
            relationship: rev_rel,
        })?;
    }
    s.broadcast(MessageSync::RelationshipDelete {
        user_id: auth_user_id,
    })?;
    Ok(StatusCode::NO_CONTENT)
}

/// Relationship list
///
/// List relationships with other users.
// TODO: Passing in someone else's id lists mutual friends
// TODO: filtering for a specific relationship type
#[utoipa::path(
    get,
    path = "/user/{user_id}/relationship",
    params(
        PaginationQuery<UserId>,
        ("user_id", description = "User id to list relationships from"),
    ),
    tags = ["relationship"],
    responses(
        (status = OK, body = PaginationResponse<RelationshipWithUserId>, description = "success"),
    )
)]
#[deprecated = "use /friend or /block for listing"]
async fn relationship_list(
    Path(_target_user_id): Path<UserIdReq>,
    Auth(auth_user_id): Auth,
    Query(q): Query<PaginationQuery<UserId>>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let rels = data.user_relationship_list(auth_user_id, q).await?;
    Ok(Json(rels))
}

/// Friend list (TODO)
///
/// List (mutual) friends.
#[utoipa::path(
    get,
    path = "/user/{user_id}/friend",
    params(
        PaginationQuery<UserId>,
        ("user_id", description = "User id to list friends from"),
    ),
    tags = ["relationship"],
    responses(
        (status = OK, body = PaginationResponse<RelationshipWithUserId>, description = "success"),
    )
)]
async fn friend_list() -> Result<()> {
    Err(Error::Unimplemented)
}

/// Friend add (TODO)
///
/// Send or accept a friend request.
#[utoipa::path(
    put,
    path = "/user/@self/friend/{target_id}",
    params(("target_id", description = "Target user's id")),
    tags = ["relationship"],
    responses((status = NO_CONTENT, description = "success"))
)]
async fn friend_add() -> Result<()> {
    Err(Error::Unimplemented)
}

/// Friend remove (TODO)
///
/// Remove friend or reject a friend request.
#[utoipa::path(
    delete,
    path = "/user/@self/friend/{target_id}",
    params(("target_id", description = "Target user's id")),
    tags = ["relationship"],
    responses((status = NO_CONTENT, description = "success"))
)]
async fn friend_remove() -> Result<()> {
    Err(Error::Unimplemented)
}

/// Block list (TODO)
///
/// List (mutually?) blocked users.
#[utoipa::path(
    get,
    path = "/user/{user_id}/block",
    params(
        PaginationQuery<UserId>,
        ("user_id", description = "User id to list blocks from"),
    ),
    tags = ["relationship"],
    responses(
        (status = OK, body = PaginationResponse<RelationshipWithUserId>, description = "success"),
    )
)]
async fn block_list() -> Result<()> {
    Err(Error::Unimplemented)
}

/// Block add (TODO)
///
/// Block a user.
#[utoipa::path(
    put,
    path = "/user/@self/block/{target_id}",
    params(("target_id", description = "Target user's id")),
    tags = ["relationship"],
    responses((status = NO_CONTENT, description = "success"))
)]
async fn block_add() -> Result<()> {
    Err(Error::Unimplemented)
}

/// Block remove (TODO)
///
/// Unblock a user.
#[utoipa::path(
    delete,
    path = "/user/@self/block/{target_id}",
    params(("target_id", description = "Target user's id")),
    tags = ["relationship"],
    responses((status = NO_CONTENT, description = "success"))
)]
async fn block_remove() -> Result<()> {
    Err(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(relationship_get))
        .routes(routes!(relationship_update))
        .routes(routes!(relationship_reset))
        .routes(routes!(relationship_list))
        .routes(routes!(friend_list))
        .routes(routes!(friend_add))
        .routes(routes!(friend_remove))
        .routes(routes!(block_list))
        .routes(routes!(block_add))
        .routes(routes!(block_remove))
}
