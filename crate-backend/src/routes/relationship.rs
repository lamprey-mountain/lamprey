use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::{
    PaginationQuery, PaginationResponse, Relationship, RelationshipPatch, RelationshipType,
    RelationshipWithUserId, UserId,
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
    let rel = data
        .user_relationship_edit(auth_user_id, target_user_id, patch)
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
    }
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
    }
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

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(relationship_get))
        .routes(routes!(relationship_update))
        .routes(routes!(relationship_reset))
        .routes(routes!(relationship_list))
}
