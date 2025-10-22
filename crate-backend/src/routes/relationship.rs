use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::user::Ignore;
use common::v1::types::{
    AuditLogEntry, AuditLogEntryId, AuditLogEntryType, MessageSync, PaginationQuery,
    PaginationResponse, RelationshipPatch, RelationshipType, RelationshipWithUserId, UserId,
};
use http::StatusCode;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::routes::util::{AuthWithSession, HeaderReason};
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
    Auth(auth_user): Auth,
    Query(q): Query<PaginationQuery<UserId>>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let rels = data.user_relationship_list(auth_user.id, q).await?;
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
    AuthWithSession(session, auth_user): AuthWithSession,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;

    let data = s.data();

    let existing = data
        .user_relationship_get(auth_user.id, target_user_id)
        .await?;

    let reverse = data
        .user_relationship_get(target_user_id, auth_user.id)
        .await?;

    match (
        existing.as_ref().and_then(|r| r.relation.as_ref()),
        reverse.as_ref().and_then(|r| r.relation.as_ref()),
    ) {
        (Some(RelationshipType::Incoming), Some(RelationshipType::Outgoing)) => {
            // accept friend request
            data.user_relationship_edit(
                auth_user.id,
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
                auth_user.id,
                RelationshipPatch {
                    note: None,
                    petname: None,
                    ignore: None,
                    relation: Some(Some(RelationshipType::Friend)),
                },
            )
            .await?;
            s.audit_log_append(AuditLogEntry {
                id: AuditLogEntryId::new(),
                room_id: auth_user.id.into_inner().into(),
                user_id: auth_user.id,
                session_id: Some(session.id),
                reason,
                ty: AuditLogEntryType::FriendAccept {
                    user_id: target_user_id,
                },
            })
            .await?;
        }
        (_, Some(RelationshipType::Block)) => return Err(Error::Blocked),
        (None, None) => {
            // send friend request
            data.user_relationship_edit(
                auth_user.id,
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
                auth_user.id,
                RelationshipPatch {
                    note: None,
                    petname: None,
                    ignore: None,
                    relation: Some(Some(RelationshipType::Incoming)),
                },
            )
            .await?;
            s.audit_log_append(AuditLogEntry {
                id: AuditLogEntryId::new(),
                room_id: auth_user.id.into_inner().into(),
                user_id: auth_user.id,
                session_id: Some(session.id),
                reason,
                ty: AuditLogEntryType::FriendRequest {
                    user_id: target_user_id,
                },
            })
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

    if let Some(rel) = data
        .user_relationship_get(auth_user.id, target_user_id)
        .await?
    {
        s.broadcast(MessageSync::RelationshipUpsert {
            user_id: auth_user.id,
            target_user_id,
            relationship: rel,
        })?;
    }

    if let Some(rel) = data
        .user_relationship_get(target_user_id, auth_user.id)
        .await?
    {
        s.broadcast(MessageSync::RelationshipUpsert {
            user_id: target_user_id,
            target_user_id: auth_user.id,
            relationship: rel,
        })?;
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
    AuthWithSession(session, auth_user): AuthWithSession,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;

    let data = s.data();

    let existing = data
        .user_relationship_get(auth_user.id, target_user_id)
        .await?;

    match existing.as_ref().and_then(|r| r.relation.as_ref()) {
        r @ Some(RelationshipType::Friend)
        | r @ Some(RelationshipType::Incoming)
        | r @ Some(RelationshipType::Outgoing) => {
            data.user_relationship_delete(auth_user.id, target_user_id)
                .await?;
            s.broadcast(MessageSync::RelationshipDelete {
                user_id: auth_user.id,
                target_user_id,
            })?;

            if r == Some(&RelationshipType::Friend) {
                s.audit_log_append(AuditLogEntry {
                    id: AuditLogEntryId::new(),
                    room_id: auth_user.id.into_inner().into(),
                    user_id: auth_user.id,
                    session_id: Some(session.id),
                    reason,
                    ty: AuditLogEntryType::FriendDelete {
                        user_id: target_user_id,
                    },
                })
                .await?;
            }

            if let Some(r) = data
                .user_relationship_get(target_user_id, auth_user.id)
                .await?
            {
                match r.relation {
                    Some(RelationshipType::Friend)
                    | Some(RelationshipType::Incoming)
                    | Some(RelationshipType::Outgoing) => {
                        data.user_relationship_delete(target_user_id, auth_user.id)
                            .await?;

                        s.broadcast(MessageSync::RelationshipDelete {
                            user_id: target_user_id,
                            target_user_id: auth_user.id,
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
    Auth(auth_user): Auth,
    Query(q): Query<PaginationQuery<UserId>>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let rels = data.user_relationship_list(auth_user.id, q).await?;
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
    AuthWithSession(session, auth_user): AuthWithSession,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    let data = s.data();

    data.user_relationship_edit(
        auth_user.id,
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
        .user_relationship_get(target_user_id, auth_user.id)
        .await?;
    if !matches!(
        reverse.as_ref().and_then(|r| r.relation.as_ref()),
        Some(&RelationshipType::Block)
    ) {
        if reverse.is_some() {
            data.user_relationship_delete(target_user_id, auth_user.id)
                .await?;
            s.broadcast(MessageSync::RelationshipDelete {
                user_id: target_user_id,
                target_user_id: auth_user.id,
            })?;
        }
    }

    let rel = data
        .user_relationship_get(auth_user.id, target_user_id)
        .await?
        .unwrap();

    s.broadcast(MessageSync::RelationshipUpsert {
        user_id: auth_user.id,
        target_user_id,
        relationship: rel,
    })?;

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: auth_user.id.into_inner().into(),
        user_id: auth_user.id,
        session_id: Some(session.id),
        reason,
        ty: AuditLogEntryType::BlockCreate {
            user_id: target_user_id,
        },
    })
    .await?;

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
    AuthWithSession(session, auth_user): AuthWithSession,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    let data = s.data();

    let existing = data
        .user_relationship_get(auth_user.id, target_user_id)
        .await?;

    if existing
        .as_ref()
        .is_some_and(|r| r.relation == Some(RelationshipType::Block))
    {
        data.user_relationship_delete(auth_user.id, target_user_id)
            .await?;

        s.broadcast(MessageSync::RelationshipDelete {
            user_id: auth_user.id,
            target_user_id,
        })?;

        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id: auth_user.id.into_inner().into(),
            user_id: auth_user.id,
            session_id: Some(session.id),
            reason,
            ty: AuditLogEntryType::BlockDelete {
                user_id: target_user_id,
            },
        })
        .await?;
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Ignore list
///
/// List ignored users.
#[utoipa::path(
    get,
    path = "/user/{user_id}/ignore",
    params(
        PaginationQuery<UserId>,
        ("user_id", description = "User id to list ignored users from"),
    ),
    tags = ["relationship"],
    responses(
        (status = OK, body = PaginationResponse<RelationshipWithUserId>, description = "success"),
    )
)]
async fn ignore_list(
    Auth(auth_user): Auth,
    Query(q): Query<PaginationQuery<UserId>>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let mut rels = data.user_relationship_list(auth_user.id, q).await?;
    rels.items.retain(|r| r.inner.ignore.is_some());
    Ok(Json(rels))
}

/// Ignore add
///
/// Ignore a user.
#[utoipa::path(
    put,
    path = "/user/@self/ignore/{target_id}",
    params(("target_id", description = "Target user's id")),
    tags = ["relationship"],
    responses((status = NO_CONTENT, description = "success"))
)]
async fn ignore_add(
    Path(target_user_id): Path<UserId>,
    AuthWithSession(session, auth_user): AuthWithSession,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(ignore): Json<Ignore>,
) -> Result<impl IntoResponse> {
    let data = s.data();

    data.user_relationship_edit(
        auth_user.id,
        target_user_id,
        RelationshipPatch {
            note: None,
            petname: None,
            ignore: Some(Some(ignore)),
            relation: None,
        },
    )
    .await?;

    let rel = data
        .user_relationship_get(auth_user.id, target_user_id)
        .await?
        .unwrap();

    s.broadcast(MessageSync::RelationshipUpsert {
        user_id: auth_user.id,
        target_user_id,
        relationship: rel,
    })?;

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: auth_user.id.into_inner().into(),
        user_id: auth_user.id,
        session_id: Some(session.id),
        reason,
        ty: AuditLogEntryType::IgnoreAdd {
            user_id: target_user_id,
        },
    })
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Ignore remove
///
/// Unignore a user.
#[utoipa::path(
    delete,
    path = "/user/@self/ignore/{target_id}",
    params(("target_id", description = "Target user's id")),
    tags = ["relationship"],
    responses((status = NO_CONTENT, description = "success"))
)]
async fn ignore_remove(
    Path(target_user_id): Path<UserId>,
    AuthWithSession(session, auth_user): AuthWithSession,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    let data = s.data();

    let existing = data
        .user_relationship_get(auth_user.id, target_user_id)
        .await?;

    if existing.as_ref().is_some_and(|r| r.ignore.is_some()) {
        data.user_relationship_edit(
            auth_user.id,
            target_user_id,
            RelationshipPatch {
                note: None,
                petname: None,
                ignore: Some(None),
                relation: None,
            },
        )
        .await?;

        let mut updated_rel = existing.unwrap();
        updated_rel.ignore = None;

        s.broadcast(MessageSync::RelationshipUpsert {
            user_id: auth_user.id,
            target_user_id,
            relationship: updated_rel,
        })?;

        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id: auth_user.id.into_inner().into(),
            user_id: auth_user.id,
            session_id: Some(session.id),
            reason,
            ty: AuditLogEntryType::IgnoreRemove {
                user_id: target_user_id,
            },
        })
        .await?;
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
        .routes(routes!(ignore_list))
        .routes(routes!(ignore_add))
        .routes(routes!(ignore_remove))
}
