use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::routes;
use common::v1::types::application::Scope;
use common::v1::types::redex::{
    EvalInput, Redex, RedexFormat, RedexLocation, RedexLocationUpdate, RedexMetadata, RedexStatus,
    RedexVersion, RedexVersionStatus,
};
use common::v1::types::util::{Changes, Time};
use common::v1::types::{
    AuditLogEntryType, ChannelType, MessageSync, Permission, RedexId, RedexVerId, RoomFeature,
};
use common::v2::types::media::MediaReference;
use http::StatusCode;
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;

use crate::consts::MAX_SCRIPT_FILE_SIZE;
use crate::error::Result;
use crate::{routes2, Error, ServerState};

use super::util::{Auth, Auth3};

/// Redex create
#[handler(routes::redex_create)]
async fn redex_create(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::redex_create::Request,
) -> Result<impl IntoResponse> {
    if !s.config.scripts.enabled {
        return Err(Error::Unimplemented);
    }

    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    let chan = srv.channels.get(req.channel_id, None).await?;
    chan.ty.ensure_has_scripts()?;

    let room_id = chan
        .room_id
        .ok_or(Error::BadStatic("channel is not in a room"))?;

    let room = srv.rooms.load_room(room_id, false).await?;
    room.ensure_feature(&RoomFeature::Scripts)?;

    let al = auth.audit_log(room_id);

    let media = match &req.redex.location {
        RedexLocationUpdate::Local { .. } => return Err(Error::Unimplemented),
        RedexLocationUpdate::Remote { .. } => return Err(Error::Unimplemented),
        RedexLocationUpdate::Hosted { media_reference } => match media_reference {
            MediaReference::Attachment { .. } => return Err(Error::Unimplemented),
            MediaReference::Url { .. } => return Err(Error::Unimplemented),
            MediaReference::Media { media_id } => {
                let mut d = s.data();
                let media = d.media_select(*media_id).await?;
                media
            }
        },
    };

    if media.size > MAX_SCRIPT_FILE_SIZE {
        return Err(Error::BadStatic("file too large"));
    }

    let redex_id = RedexId::new();
    let version_id = RedexVerId::new();
    let created_at = Time::now_utc();

    let format = req.redex.format.clone();
    let media_id = media.id;
    let location = RedexLocation::Hosted { media };

    let script = Redex {
        id: redex_id,
        channel_id: req.channel_id,
        creator_id: auth.user.id,
        created_at,
        deleted_at: None,
        latest_version: RedexVersion {
            version_id,
            created_at,
            deleted_at: None,
            format: format.clone(),
            location,
            metadata: RedexMetadata::new("unnamed".to_owned()), // will be replaced
            status: RedexVersionStatus::Processing,
        },
        status: RedexStatus::Creating,
        permissions: vec![],
        handlers: vec![],
    };

    srv.scripts.create_script(script.clone()).await?;

    al.commit_success(AuditLogEntryType::RedexCreate {
        channel_id: req.channel_id,
        redex_id,
        changes: Changes::new()
            .add(
                "format",
                &match &format {
                    RedexFormat::Javascript => "Javascript",
                    RedexFormat::Webassembly => "Webassembly",
                },
            )
            .add("location", &"hosted")
            .add("media_id", &media_id)
            .build(),
    })
    .await?;

    s.broadcast_room(
        room_id,
        auth.user.id,
        MessageSync::ScriptCreate {
            script: script.clone(),
        },
    )
    .await?;

    Ok((StatusCode::CREATED, Json(script)))
}

/// Redex list
#[handler(routes::redex_list)]
async fn redex_list(
    auth: Auth3,
    State(s): State<Arc<ServerState>>,
    req: routes::redex_list::Request,
) -> Result<impl IntoResponse> {
    if !s.config.scripts.enabled {
        return Err(Error::Unimplemented);
    }

    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    let chan = srv.channels.get(req.channel_id, None).await?;
    let room_id = chan
        .room_id
        .ok_or(Error::BadStatic("channel is not in a room"))?;

    let room = srv.rooms.load_room(room_id, false).await?;
    room.ensure_feature(&RoomFeature::Scripts)?;

    srv.perms
        .for_channel3(auth.user_id(), req.channel_id)
        .await?
        .ensure_view()?
        .check()?;

    let scripts = s
        .data()
        .script_list_by_channel(req.channel_id, req.pagination)
        .await?;

    Ok(Json(scripts))
}

/// Redex get
#[handler(routes::redex_get)]
async fn redex_get(
    auth: Auth3,
    State(s): State<Arc<ServerState>>,
    req: routes::redex_get::Request,
) -> Result<impl IntoResponse> {
    if !s.config.scripts.enabled {
        return Err(Error::Unimplemented);
    }

    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    let chan = srv.channels.get(req.channel_id, None).await?;
    let room_id = chan
        .room_id
        .ok_or(Error::BadStatic("channel is not in a room"))?;

    let room = srv.rooms.load_room(room_id, false).await?;
    room.ensure_feature(&RoomFeature::Scripts)?;

    srv.perms
        .for_channel3(auth.user_id(), req.channel_id)
        .await?
        .ensure_view()?
        .check()?;

    let script = s
        .data()
        .script_get(req.redex_id)
        .await?
        .ok_or(Error::NotFound)?;

    if script.channel_id != req.channel_id {
        return Err(Error::NotFound);
    }

    Ok((StatusCode::OK, Json(script)))
}

/// Redex delete
#[handler(routes::redex_delete)]
async fn redex_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::redex_delete::Request,
) -> Result<impl IntoResponse> {
    if !s.config.scripts.enabled {
        return Err(Error::Unimplemented);
    }

    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    let chan = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    let room_id = chan
        .room_id
        .ok_or(Error::BadStatic("channel is not in a room"))?;

    let room = srv.rooms.load_room(room_id, false).await?;
    room.ensure_feature(&RoomFeature::Scripts)?;

    let al = auth.audit_log(room_id);

    s.data().script_delete(req.redex_id).await?;

    // TODO: remove script from service

    al.commit_success(AuditLogEntryType::RedexDelete {
        channel_id: req.channel_id,
        redex_id: req.redex_id,
        changes: vec![], // TODO: populate changes
    })
    .await?;

    s.broadcast_room(
        room_id,
        auth.user.id,
        MessageSync::ScriptDelete {
            channel_id: req.channel_id,
            redex_id: req.redex_id,
        },
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Redex content update
#[handler(routes::redex_content_update)]
async fn redex_content_update(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::redex_content_update::Request,
) -> Result<impl IntoResponse> {
    if !s.config.scripts.enabled {
        return Err(Error::Unimplemented);
    }

    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    let chan = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    let room_id = chan
        .room_id
        .ok_or(Error::BadStatic("channel is not in a room"))?;

    let room = srv.rooms.load_room(room_id, false).await?;
    room.ensure_feature(&RoomFeature::Scripts)?;

    let al = auth.audit_log(room_id);

    let script = s
        .data()
        .script_get(req.redex_id)
        .await?
        .ok_or(Error::NotFound)?;

    if script.channel_id != req.channel_id {
        return Err(Error::NotFound);
    }

    let media = match &req.content.location {
        RedexLocationUpdate::Local { .. } => return Err(Error::Unimplemented),
        RedexLocationUpdate::Remote { .. } => return Err(Error::Unimplemented),
        RedexLocationUpdate::Hosted { media_reference } => match media_reference {
            MediaReference::Attachment { .. } => return Err(Error::Unimplemented),
            MediaReference::Url { .. } => return Err(Error::Unimplemented),
            MediaReference::Media { media_id } => {
                let mut d = s.data();
                let media = d.media_select(*media_id).await?;
                media
            }
        },
    };

    if media.size > MAX_SCRIPT_FILE_SIZE {
        return Err(Error::BadStatic("file too large"));
    }

    let version_id = RedexVerId::new();
    let created_at = Time::now_utc();
    let format = req.content.format.clone();
    let media_id = media.id;
    let location = RedexLocation::Hosted { media };

    let new_version = RedexVersion {
        version_id,
        created_at,
        deleted_at: None,
        format: format.clone(),
        location,
        metadata: RedexMetadata::new("unnamed".to_owned()), // will be replaced during process
        status: RedexVersionStatus::Processing,
    };

    srv.scripts
        .create_script_version(script.clone(), new_version.clone())
        .await?;

    al.commit_success(AuditLogEntryType::RedexVersionCreate {
        channel_id: req.channel_id,
        redex_id: req.redex_id,
        redex_version_id: version_id,
        changes: Changes::new()
            .add(
                "format",
                &match &format {
                    RedexFormat::Javascript => "Javascript",
                    RedexFormat::Webassembly => "Webassembly",
                },
            )
            .add("location", &"hosted")
            .add("media_id", &media_id)
            .build(),
    })
    .await?;

    Ok((StatusCode::OK, Json(new_version)))
}

/// Redex trigger
#[handler(routes::redex_trigger)]
async fn redex_trigger(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::redex_trigger::Request,
) -> Result<impl IntoResponse> {
    if !s.config.scripts.enabled {
        return Err(Error::Unimplemented);
    }

    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();

    let chan = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    if chan.ty != ChannelType::Scripts {
        return Err(Error::BadStatic("channel is not a scripts channel"));
    }
    let room_id = chan
        .room_id
        .ok_or(Error::BadStatic("channel is not in a room"))?;

    let room = srv.rooms.load_room(room_id, false).await?;
    room.ensure_feature(&RoomFeature::Scripts)?;

    srv.perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .check()?;

    let redex_version_id = srv.scripts.get_redex_version_id(req.redex_id).await?;

    let run_ctl = srv
        .scripts
        .spawn(
            req.channel_id,
            req.redex_id,
            redex_version_id,
            EvalInput::Manual {
                id: req.run.trigger_id,
                user_id: auth.user.id,
            },
        )
        .await?;
    let run = run_ctl.eval().to_owned();

    s.broadcast_room(
        room_id,
        auth.user.id,
        MessageSync::ScriptRunCreate {
            channel_id: req.channel_id,
            run: run.clone(),
        },
    )
    .await?;

    Ok((StatusCode::CREATED, Json(run)))
}

/// Redex version list
#[handler(routes::redex_version_list)]
async fn redex_version_list(
    auth: Auth3,
    State(s): State<Arc<ServerState>>,
    req: routes::redex_version_list::Request,
) -> Result<impl IntoResponse> {
    if !s.config.scripts.enabled {
        return Err(Error::Unimplemented);
    }

    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    let chan = srv.channels.get(req.channel_id, None).await?;
    let room_id = chan
        .room_id
        .ok_or(Error::BadStatic("channel is not in a room"))?;

    let room = srv.rooms.load_room(room_id, false).await?;
    room.ensure_feature(&RoomFeature::Scripts)?;

    srv.perms
        .for_channel3(auth.user_id(), req.channel_id)
        .await?
        .ensure_view()?
        .check()?;

    let versions = s
        .data()
        .script_version_list_by_script(req.channel_id, req.redex_id, req.pagination)
        .await?;

    Ok(Json(versions))
}

/// Redex version get
#[handler(routes::redex_version_get)]
async fn redex_version_get(
    auth: Auth3,
    State(s): State<Arc<ServerState>>,
    req: routes::redex_version_get::Request,
) -> Result<impl IntoResponse> {
    if !s.config.scripts.enabled {
        return Err(Error::Unimplemented);
    }

    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    let chan = srv.channels.get(req.channel_id, None).await?;
    let room_id = chan
        .room_id
        .ok_or(Error::BadStatic("channel is not in a room"))?;

    let room = srv.rooms.load_room(room_id, false).await?;
    room.ensure_feature(&RoomFeature::Scripts)?;

    srv.perms
        .for_channel3(auth.user_id(), req.channel_id)
        .await?
        .ensure_view()?
        .check()?;

    let version = s
        .data()
        .script_version_get(req.redex_id, req.channel_id, req.version_id)
        .await?
        .ok_or(Error::NotFound)?;

    Ok((StatusCode::OK, Json(version)))
}

/// Redex version delete
#[handler(routes::redex_version_delete)]
async fn redex_version_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::redex_version_delete::Request,
) -> Result<impl IntoResponse> {
    if !s.config.scripts.enabled {
        return Err(Error::Unimplemented);
    }

    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    let chan = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    let room_id = chan
        .room_id
        .ok_or(Error::BadStatic("channel is not in a room"))?;

    let room = srv.rooms.load_room(room_id, false).await?;
    room.ensure_feature(&RoomFeature::Scripts)?;

    let al = auth.audit_log(room_id);

    // TODO: verify the version exists and belongs to this script

    s.data()
        .script_version_delete(req.redex_id, req.version_id)
        .await?;

    al.commit_success(AuditLogEntryType::RedexVersionDelete {
        channel_id: req.channel_id,
        redex_id: req.redex_id,
        redex_version_id: req.version_id,
        changes: Changes::new().build(), // TODO: add metadata to changes
    })
    .await?;

    s.broadcast_room(
        room_id,
        auth.user.id,
        MessageSync::ScriptVersionDelete {
            channel_id: req.channel_id,
            redex_id: req.redex_id,
            version_id: req.version_id,
        },
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Redex version restore
#[handler(routes::redex_version_restore)]
async fn redex_version_restore(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::redex_version_restore::Request,
) -> Result<impl IntoResponse> {
    if !s.config.scripts.enabled {
        return Err(Error::Unimplemented);
    }

    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    let chan = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    let room_id = chan
        .room_id
        .ok_or(Error::BadStatic("channel is not in a room"))?;

    let room = srv.rooms.load_room(room_id, false).await?;
    room.ensure_feature(&RoomFeature::Scripts)?;

    let _al = auth.audit_log(room_id);

    // TODO: do i soft-undelete the version (set deleted_at to None)? or do i create a new version with the same content as the old version?
    // TODO: verify the version exists and belongs to this script
    // TODO: make the script service reload the script

    Ok(Error::Unimplemented)
}

/// Redex dependency graph
#[handler(routes::redex_depends)]
async fn redex_depends(
    auth: Auth3,
    State(s): State<Arc<ServerState>>,
    req: routes::redex_depends::Request,
) -> Result<impl IntoResponse> {
    if !s.config.scripts.enabled {
        return Err(Error::Unimplemented);
    }

    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    let chan = srv.channels.get(req.channel_id, None).await?;
    let room_id = chan
        .room_id
        .ok_or(Error::BadStatic("channel is not in a room"))?;

    let room = srv.rooms.load_room(room_id, false).await?;
    room.ensure_feature(&RoomFeature::Scripts)?;

    srv.perms
        .for_channel3(auth.user_id(), req.channel_id)
        .await?
        .ensure_view()?
        .check()?;

    Ok(Error::Unimplemented)
}

/// Redex dependency update
#[handler(routes::redex_depends_update)]
async fn redex_depends_update(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::redex_depends_update::Request,
) -> Result<impl IntoResponse> {
    if !s.config.scripts.enabled {
        return Err(Error::Unimplemented);
    }

    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    let chan = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    let room_id = chan
        .room_id
        .ok_or(Error::BadStatic("channel is not in a room"))?;

    let room = srv.rooms.load_room(room_id, false).await?;
    room.ensure_feature(&RoomFeature::Scripts)?;

    let _al = auth.audit_log(room_id);

    Ok(Error::Unimplemented)
}

/// Redex run list
#[handler(routes::redex_eval_list)]
async fn redex_eval_list(
    auth: Auth3,
    State(s): State<Arc<ServerState>>,
    req: routes::redex_eval_list::Request,
) -> Result<impl IntoResponse> {
    if !s.config.scripts.enabled {
        return Err(Error::Unimplemented);
    }

    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    let chan = srv.channels.get(req.channel_id, None).await?;
    let room_id = chan
        .room_id
        .ok_or(Error::BadStatic("channel is not in a room"))?;

    let room = srv.rooms.load_room(room_id, false).await?;
    room.ensure_feature(&RoomFeature::Scripts)?;

    let runs = s
        .data()
        .script_run_list(req.redex_id, req.pagination)
        .await?;

    Ok(Json(runs))
}

/// Redex run get
#[handler(routes::redex_eval_get)]
async fn redex_eval_get(
    auth: Auth3,
    State(s): State<Arc<ServerState>>,
    req: routes::redex_eval_get::Request,
) -> Result<impl IntoResponse> {
    if !s.config.scripts.enabled {
        return Err(Error::Unimplemented);
    }

    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    let chan = srv.channels.get(req.channel_id, None).await?;
    let room_id = chan
        .room_id
        .ok_or(Error::BadStatic("channel is not in a room"))?;

    let room = srv.rooms.load_room(room_id, false).await?;
    room.ensure_feature(&RoomFeature::Scripts)?;

    let run = s
        .data()
        .script_run_get(req.eval_id)
        .await?
        .ok_or(Error::NotFound)?;

    Ok(Json(run))
}

/// Redex run stop
#[handler(routes::redex_eval_stop)]
async fn redex_eval_stop(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::redex_eval_stop::Request,
) -> Result<impl IntoResponse> {
    if !s.config.scripts.enabled {
        return Err(Error::Unimplemented);
    }

    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    let chan = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    let room_id = chan
        .room_id
        .ok_or(Error::BadStatic("channel is not in a room"))?;

    let room = srv.rooms.load_room(room_id, false).await?;
    room.ensure_feature(&RoomFeature::Scripts)?;

    srv.perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .needs(Permission::ScriptManage)
        .check()?;

    srv.scripts
        .stop_run(req.channel_id, req.redex_id, req.eval_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Redex run log
#[handler(routes::redex_eval_log)]
async fn redex_eval_log(
    auth: Auth3,
    State(s): State<Arc<ServerState>>,
    req: routes::redex_eval_log::Request,
) -> Result<impl IntoResponse> {
    if !s.config.scripts.enabled {
        return Err(Error::Unimplemented);
    }

    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    let chan = srv.channels.get(req.channel_id, None).await?;
    let room_id = chan
        .room_id
        .ok_or(Error::BadStatic("channel is not in a room"))?;

    let room = srv.rooms.load_room(room_id, false).await?;
    room.ensure_feature(&RoomFeature::Scripts)?;

    srv.perms
        .for_channel3(auth.user_id(), req.channel_id)
        .await?
        .ensure_view()?
        .check()?;

    let logs = s
        .data()
        .script_log_list(req.eval_id, req.pagination)
        .await?;

    Ok(Json(logs))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(redex_create))
        .routes(routes2!(redex_list))
        .routes(routes2!(redex_get))
        .routes(routes2!(redex_delete))
        .routes(routes2!(redex_content_update))
        .routes(routes2!(redex_trigger))
        .routes(routes2!(redex_version_list))
        .routes(routes2!(redex_version_get))
        .routes(routes2!(redex_version_delete))
        .routes(routes2!(redex_version_restore))
        .routes(routes2!(redex_depends))
        .routes(routes2!(redex_depends_update))
        .routes(routes2!(redex_eval_list))
        .routes(routes2!(redex_eval_get))
        .routes(routes2!(redex_eval_stop))
        .routes(routes2!(redex_eval_log))
}
