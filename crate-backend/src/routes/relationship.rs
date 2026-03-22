use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::routes;
use common::v1::types::application::Scope;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::user::Ignore;
use common::v1::types::util::Time;
use common::v1::types::{
    AuditLogEntryType, MessageSync, Permission, RelationshipPatch, RelationshipType, UserId,
};
use http::StatusCode;
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;

use crate::error::{Error, Result};
use crate::routes::util::Auth;
use crate::{routes2, ServerState};

/// Friend list
///
/// List (mutual) friends.
#[handler(routes::friend_list)]
async fn friend_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::friend_list::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let data = s.data();
    let rels = data
        .user_relationship_list_friends(auth.user.id, req.pagination)
        .await?;
    Ok(Json(rels))
}

/// Friend list pending
///
/// List pending friend requests (both incoming and outgoing).
#[handler(routes::friend_list_pending)]
async fn friend_list_pending(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::friend_list_pending::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let data = s.data();
    let rels = data
        .user_relationship_list_pending(auth.user.id, req.pagination)
        .await?;
    Ok(Json(rels))
}

/// Friend add
///
/// Send or accept a friend request.
#[handler(routes::friend_add)]
async fn friend_add(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::friend_add::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    let data = s.data();
    let srv = s.services();

    let target_user = data.user_get(req.target_id).await?;
    if !target_user.can_friend() {
        return Err(ApiError::from_code(ErrorCode::CannotFriendThisUser).into());
    }

    let existing = data
        .user_relationship_get(auth.user.id, req.target_id)
        .await?;

    let reverse = data
        .user_relationship_get(req.target_id, auth.user.id)
        .await?;

    match (
        existing.as_ref().and_then(|r| r.relation.as_ref()),
        reverse.as_ref().and_then(|r| r.relation.as_ref()),
    ) {
        (Some(RelationshipType::Incoming), Some(RelationshipType::Outgoing)) => {
            // accept friend request
            data.user_relationship_edit(
                auth.user.id,
                req.target_id,
                RelationshipPatch {
                    ignore: None,
                    relation: Some(Some(RelationshipType::Friend)),
                },
            )
            .await?;
            data.user_relationship_edit(
                req.target_id,
                auth.user.id,
                RelationshipPatch {
                    ignore: None,
                    relation: Some(Some(RelationshipType::Friend)),
                },
            )
            .await?;
            let al = auth.audit_log(auth.user.id.into_inner().into());
            al.commit_success(AuditLogEntryType::FriendAccept {
                user_id: req.target_id,
            })
            .await?;
        }
        (_, Some(RelationshipType::Block)) => return Err(Error::Blocked),
        (None, None) => {
            srv.perms
                .for_server(auth.user.id)
                .await?
                .ensure(Permission::FriendCreate)?;

            let target_prefs = srv.cache.preferences_get(req.target_id).await?;
            let friends_prefs = &target_prefs.privacy.friends;

            if let Some(pause_until) = &friends_prefs.pause_until {
                if pause_until > &Time::now_utc() {
                    return Err(ApiError::from_code(ErrorCode::FriendRequestsPaused).into());
                }
            }

            let allowed = if friends_prefs.allow_everyone {
                true
            } else {
                let mutual_room_allowed = if friends_prefs.allow_mutual_room {
                    srv.perms
                        .allows_friend_request_from_user(auth.user.id, req.target_id)
                        .await?
                } else {
                    false
                };

                let mutual_friend_allowed = friends_prefs.allow_mutual_friend
                    && data
                        .user_has_mutual_friend(auth.user.id, req.target_id)
                        .await?;

                mutual_room_allowed || mutual_friend_allowed
            };

            if !allowed {
                return Err(ApiError::from_code(ErrorCode::InvalidData).into());
            }

            data.user_relationship_edit(
                auth.user.id,
                req.target_id,
                RelationshipPatch {
                    ignore: None,
                    relation: Some(Some(RelationshipType::Outgoing)),
                },
            )
            .await?;
            data.user_relationship_edit(
                req.target_id,
                auth.user.id,
                RelationshipPatch {
                    ignore: None,
                    relation: Some(Some(RelationshipType::Incoming)),
                },
            )
            .await?;
            let al = auth.audit_log(auth.user.id.into_inner().into());
            al.commit_success(AuditLogEntryType::FriendRequest {
                user_id: req.target_id,
            })
            .await?;
        }
        (Some(RelationshipType::Friend), Some(RelationshipType::Friend)) => {
            return Ok(StatusCode::NO_CONTENT);
        }
        (Some(RelationshipType::Outgoing), Some(RelationshipType::Incoming)) => {
            return Ok(StatusCode::NO_CONTENT);
        }
        (Some(RelationshipType::Block), _) => {
            return Err(ApiError::from_code(ErrorCode::UnblockUserFirst).into());
        }
        _ => unreachable!("invalid data in database?"),
    }

    if let Some(rel) = data
        .user_relationship_get(auth.user.id, req.target_id)
        .await?
    {
        s.broadcast(MessageSync::RelationshipUpsert {
            user_id: auth.user.id,
            target_user_id: req.target_id,
            relationship: rel,
        })?;
    }

    if let Some(rel) = data
        .user_relationship_get(req.target_id, auth.user.id)
        .await?
    {
        s.broadcast(MessageSync::RelationshipUpsert {
            user_id: req.target_id,
            target_user_id: auth.user.id,
            relationship: rel,
        })?;
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Friend remove
///
/// Remove friend or reject a friend request.
#[handler(routes::friend_remove)]
async fn friend_remove(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::friend_remove::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    let data = s.data();

    let existing = data
        .user_relationship_get(auth.user.id, req.target_id)
        .await?;

    match existing.as_ref().and_then(|r| r.relation.as_ref()) {
        r @ Some(RelationshipType::Friend)
        | r @ Some(RelationshipType::Incoming)
        | r @ Some(RelationshipType::Outgoing) => {
            data.user_relationship_delete(auth.user.id, req.target_id)
                .await?;
            s.broadcast(MessageSync::RelationshipDelete {
                user_id: auth.user.id,
                target_user_id: req.target_id,
            })?;

            if r == Some(&RelationshipType::Friend) {
                let al = auth.audit_log(auth.user.id.into_inner().into());
                al.commit_success(AuditLogEntryType::FriendDelete {
                    user_id: req.target_id,
                })
                .await?;
            }

            if let Some(r) = data
                .user_relationship_get(req.target_id, auth.user.id)
                .await?
            {
                match r.relation {
                    Some(RelationshipType::Friend)
                    | Some(RelationshipType::Incoming)
                    | Some(RelationshipType::Outgoing) => {
                        data.user_relationship_delete(req.target_id, auth.user.id)
                            .await?;

                        s.broadcast(MessageSync::RelationshipDelete {
                            user_id: req.target_id,
                            target_user_id: auth.user.id,
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
#[handler(routes::block_list)]
async fn block_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::block_list::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let data = s.data();
    let rels = data
        .user_relationship_list_blocked(auth.user.id, req.pagination)
        .await?;
    Ok(Json(rels))
}

/// Block add
///
/// Block a user. Removes them as a friend if they are one.
#[handler(routes::block_add)]
async fn block_add(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::block_add::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let data = s.data();

    data.user_relationship_edit(
        auth.user.id,
        req.target_id,
        RelationshipPatch {
            ignore: None,
            relation: Some(Some(RelationshipType::Block)),
        },
    )
    .await?;

    let reverse = data
        .user_relationship_get(req.target_id, auth.user.id)
        .await?;
    if !matches!(
        reverse.as_ref().and_then(|r| r.relation.as_ref()),
        Some(&RelationshipType::Block)
    ) {
        if reverse.is_some() {
            data.user_relationship_delete(req.target_id, auth.user.id)
                .await?;
            s.broadcast(MessageSync::RelationshipDelete {
                user_id: req.target_id,
                target_user_id: auth.user.id,
            })?;
        }
    }

    let rel = data
        .user_relationship_get(auth.user.id, req.target_id)
        .await?
        .unwrap();

    s.broadcast(MessageSync::RelationshipUpsert {
        user_id: auth.user.id,
        target_user_id: req.target_id,
        relationship: rel,
    })?;

    let al = auth.audit_log(auth.user.id.into_inner().into());
    al.commit_success(AuditLogEntryType::BlockCreate {
        user_id: req.target_id,
    })
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Block remove
///
/// Unblock a user.
#[handler(routes::block_remove)]
async fn block_remove(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::block_remove::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let data = s.data();

    let existing = data
        .user_relationship_get(auth.user.id, req.target_id)
        .await?;

    if existing
        .as_ref()
        .is_some_and(|r| r.relation == Some(RelationshipType::Block))
    {
        data.user_relationship_delete(auth.user.id, req.target_id)
            .await?;

        s.broadcast(MessageSync::RelationshipDelete {
            user_id: auth.user.id,
            target_user_id: req.target_id,
        })?;

        let al = auth.audit_log(auth.user.id.into_inner().into());
        al.commit_success(AuditLogEntryType::BlockDelete {
            user_id: req.target_id,
        })
        .await?;
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Ignore list
///
/// List ignored users.
#[handler(routes::ignore_list)]
async fn ignore_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::ignore_list::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let data = s.data();
    let rels = data
        .user_relationship_list_ignored(auth.user.id, req.pagination)
        .await?;
    Ok(Json(rels))
}

/// Ignore add
///
/// Ignore a user.
#[handler(routes::ignore_add)]
async fn ignore_add(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::ignore_add::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let data = s.data();

    data.user_relationship_edit(
        auth.user.id,
        req.target_id,
        RelationshipPatch {
            ignore: Some(Some(req.ignore)),
            relation: None,
        },
    )
    .await?;

    let rel = data
        .user_relationship_get(auth.user.id, req.target_id)
        .await?
        .unwrap();

    s.broadcast(MessageSync::RelationshipUpsert {
        user_id: auth.user.id,
        target_user_id: req.target_id,
        relationship: rel,
    })?;

    let al = auth.audit_log(auth.user.id.into_inner().into());
    al.commit_success(AuditLogEntryType::IgnoreAdd {
        user_id: req.target_id,
    })
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Ignore remove
///
/// Unignore a user.
#[handler(routes::ignore_remove)]
async fn ignore_remove(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::ignore_remove::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let data = s.data();

    let existing = data
        .user_relationship_get(auth.user.id, req.target_id)
        .await?;

    if existing.as_ref().is_some_and(|r| r.ignore.is_some()) {
        data.user_relationship_edit(
            auth.user.id,
            req.target_id,
            RelationshipPatch {
                ignore: Some(None),
                relation: None,
            },
        )
        .await?;

        let mut updated_rel = existing.unwrap();
        updated_rel.ignore = None;

        s.broadcast(MessageSync::RelationshipUpsert {
            user_id: auth.user.id,
            target_user_id: req.target_id,
            relationship: updated_rel,
        })?;

        let al = auth.audit_log(auth.user.id.into_inner().into());
        al.commit_success(AuditLogEntryType::IgnoreRemove {
            user_id: req.target_id,
        })
        .await?;
    }

    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(friend_list))
        .routes(routes2!(friend_list_pending))
        .routes(routes2!(friend_add))
        .routes(routes2!(friend_remove))
        .routes(routes2!(block_list))
        .routes(routes2!(block_add))
        .routes(routes2!(block_remove))
        .routes(routes2!(ignore_list))
        .routes(routes2!(ignore_add))
        .routes(routes2!(ignore_remove))
}
