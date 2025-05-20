use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::{
    MessageSync, PaginationQuery, PaginationResponse, RelationshipPatch, RelationshipType,
    RelationshipWithUserId, UserId,
};
use http::StatusCode;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::ServerState;

use super::util::Auth;
use crate::error::{Error, Result};

/// Friend list
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
async fn friend_list(
    Auth(auth_user_id): Auth,
    Query(q): Query<PaginationQuery<UserId>>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let rels = data.user_relationship_list(auth_user_id, q).await?;
    Ok(Json(rels))
}

/// Friend add
///
/// Send or accept a friend request.
#[utoipa::path(
    put,
    path = "/user/@self/friend/{target_id}",
    params(("target_id", description = "Target user's id")),
    tags = ["relationship"],
    responses((status = NO_CONTENT, description = "success"))
)]
async fn friend_add(
    Path(target_user_id): Path<UserId>,
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();

    let existing = data
        .user_relationship_get(auth_user_id, target_user_id)
        .await?;

    let reverse = data
        .user_relationship_get(target_user_id, auth_user_id)
        .await?;

    match (
        existing.as_ref().and_then(|r| r.relation.as_ref()),
        reverse.as_ref().and_then(|r| r.relation.as_ref()),
    ) {
        (Some(RelationshipType::Incoming), Some(RelationshipType::Outgoing)) => {
            // accept friend request
            data.user_relationship_edit(
                auth_user_id,
                target_user_id,
                RelationshipPatch {
                    note: None,
                    petname: None,
                    ignore: None,
                    relation: Some(Some(RelationshipType::Friend)),
                },
            )
            .await?;
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
        (_, Some(RelationshipType::Block)) => return Err(Error::Blocked),
        (None, None) => {
            // send friend request
            data.user_relationship_edit(
                auth_user_id,
                target_user_id,
                RelationshipPatch {
                    note: None,
                    petname: None,
                    ignore: None,
                    relation: Some(Some(RelationshipType::Outgoing)),
                },
            )
            .await?;
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
        }
        (Some(RelationshipType::Friend), Some(RelationshipType::Friend)) => {
            // already friends
            return Ok(StatusCode::NO_CONTENT);
        }
        (Some(RelationshipType::Outgoing), Some(RelationshipType::Incoming)) => {
            // already sent a request
            return Ok(StatusCode::NO_CONTENT);
        }
        (Some(RelationshipType::Block), _) => {
            // you blocked them
            return Err(Error::BadStatic("unblock this user first"));
        }
        _ => unreachable!("invalid data in database?"),
    }

    for (uid, rel) in [
        (
            auth_user_id,
            data.user_relationship_get(auth_user_id, target_user_id)
                .await?,
        ),
        (
            target_user_id,
            data.user_relationship_get(target_user_id, auth_user_id)
                .await?,
        ),
    ] {
        if let Some(rel) = rel {
            s.broadcast(MessageSync::RelationshipUpsert {
                user_id: uid,
                relationship: rel,
            })?;
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Friend remove
///
/// Remove friend or reject a friend request.
#[utoipa::path(
    delete,
    path = "/user/@self/friend/{target_id}",
    params(("target_id", description = "Target user's id")),
    tags = ["relationship"],
    responses((status = NO_CONTENT, description = "success"))
)]
async fn friend_remove(
    Path(target_user_id): Path<UserId>,
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();

    let existing = data
        .user_relationship_get(auth_user_id, target_user_id)
        .await?;

    match existing.as_ref().and_then(|r| r.relation.as_ref()) {
        Some(RelationshipType::Friend)
        | Some(RelationshipType::Incoming)
        | Some(RelationshipType::Outgoing) => {
            data.user_relationship_delete(auth_user_id, target_user_id)
                .await?;
            s.broadcast(MessageSync::RelationshipDelete {
                user_id: auth_user_id,
            })?;

            if let Some(r) = data
                .user_relationship_get(target_user_id, auth_user_id)
                .await?
            {
                match r.relation {
                    Some(RelationshipType::Friend)
                    | Some(RelationshipType::Incoming)
                    | Some(RelationshipType::Outgoing) => {
                        data.user_relationship_delete(target_user_id, auth_user_id)
                            .await?;

                        s.broadcast(MessageSync::RelationshipDelete {
                            user_id: auth_user_id,
                        })?;
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Block list
///
/// List blocked users.
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
async fn block_list(
    Auth(auth_user_id): Auth,
    Query(q): Query<PaginationQuery<UserId>>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let rels = data.user_relationship_list(auth_user_id, q).await?;
    Ok(Json(rels))
}

/// Block add
///
/// Block a user. Removes them as a friend if they are one.
#[utoipa::path(
    put,
    path = "/user/@self/block/{target_id}",
    params(("target_id", description = "Target user's id")),
    tags = ["relationship"],
    responses((status = NO_CONTENT, description = "success"))
)]
async fn block_add(
    Path(target_user_id): Path<UserId>,
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();

    data.user_relationship_edit(
        auth_user_id,
        target_user_id,
        RelationshipPatch {
            note: None,
            petname: None,
            ignore: None,
            relation: Some(Some(RelationshipType::Block)),
        },
    )
    .await?;

    let reverse = data
        .user_relationship_get(target_user_id, auth_user_id)
        .await?;
    if !matches!(
        reverse.and_then(|r| r.relation),
        Some(RelationshipType::Block)
    ) {
        data.user_relationship_delete(target_user_id, auth_user_id)
            .await?;
    }

    let rel = data
        .user_relationship_get(auth_user_id, target_user_id)
        .await?
        .unwrap();

    s.broadcast(MessageSync::RelationshipUpsert {
        user_id: auth_user_id,
        relationship: rel,
    })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Block remove
///
/// Unblock a user.
#[utoipa::path(
    delete,
    path = "/user/@self/block/{target_id}",
    params(("target_id", description = "Target user's id")),
    tags = ["relationship"],
    responses((status = NO_CONTENT, description = "success"))
)]
async fn block_remove(
    Path(target_user_id): Path<UserId>,
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();

    let existing = data
        .user_relationship_get(auth_user_id, target_user_id)
        .await?;

    if existing
        .as_ref()
        .is_some_and(|r| r.relation == Some(RelationshipType::Block))
    {
        data.user_relationship_delete(auth_user_id, target_user_id)
            .await?;

        s.broadcast(MessageSync::RelationshipDelete {
            user_id: auth_user_id,
        })?;
    }

    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(friend_list))
        .routes(routes!(friend_add))
        .routes(routes!(friend_remove))
        .routes(routes!(block_list))
        .routes(routes!(block_add))
        .routes(routes!(block_remove))
}
