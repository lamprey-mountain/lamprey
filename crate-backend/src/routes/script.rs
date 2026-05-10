use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::routes;
use common::v1::types::application::Scope;
use common::v1::types::script::{
    Script, ScriptFormat, ScriptLocation, ScriptLocationSet, ScriptMetadata, ScriptStatus,
    ScriptVersion, ScriptVersionStatus,
};
use common::v1::types::util::{Changes, Time};
use common::v1::types::{AuditLogEntryType, ChannelType, MessageSync, ScriptId, ScriptVerId};
use common::v2::types::media::MediaReference;
use http::StatusCode;
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;

use crate::consts::MAX_SCRIPT_FILE_SIZE;
use crate::error::Result;
use crate::services::scripts::ScriptInputData;
use crate::{routes2, Error, ServerState};

use super::util::{Auth, Auth3};

/// Script create
#[handler(routes::script_create)]
async fn script_create(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::script_create::Request,
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
    let al = auth.audit_log(room_id);

    match &req.script.format {
        ScriptFormat::Javascript => {}
        ScriptFormat::Webassembly => return Err(Error::Unimplemented),
    };

    let media = match &req.script.location {
        ScriptLocationSet::Local { .. } => return Err(Error::Unimplemented),
        ScriptLocationSet::Remote { .. } => return Err(Error::Unimplemented),
        ScriptLocationSet::Hosted { media_reference } => match media_reference {
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

    let script_id = ScriptId::new();
    let version_id = ScriptVerId::new();
    let created_at = Time::now_utc();

    let format = req.script.format.clone();
    let media_id = media.id;
    let location = ScriptLocation::Hosted { media };

    // TODO: extract metadata from userscript-like comments in the script source
    // alternatively, i could extract similarly to import extraction (read exported data)
    let metadata = ScriptMetadata::default();

    let script = Script {
        id: script_id,
        channel_id: req.channel_id,
        creator_id: auth.user.id,
        created_at,
        deleted_at: None,
        latest_version: ScriptVersion {
            version_id,
            created_at,
            deleted_at: None,
            format: format.clone(),
            location,
            metadata,
            status: ScriptVersionStatus::Processing,
        },
        status: ScriptStatus::Creating,
        permissions: vec![],
        inputs: vec![],
    };

    srv.scripts.create_script(script.clone()).await?;

    al.commit_success(AuditLogEntryType::ScriptCreate {
        channel_id: req.channel_id,
        script_id,
        changes: Changes::new()
            .add(
                "format",
                &match &format {
                    ScriptFormat::Javascript => "Javascript",
                    ScriptFormat::Webassembly => "Webassembly",
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

/// Script list
#[handler(routes::script_list)]
async fn script_list(
    auth: Auth3,
    State(s): State<Arc<ServerState>>,
    req: routes::script_list::Request,
) -> Result<impl IntoResponse> {
    if !s.config.scripts.enabled {
        return Err(Error::Unimplemented);
    }

    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
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

/// Script get
#[handler(routes::script_get)]
async fn script_get(
    auth: Auth3,
    State(s): State<Arc<ServerState>>,
    req: routes::script_get::Request,
) -> Result<impl IntoResponse> {
    if !s.config.scripts.enabled {
        return Err(Error::Unimplemented);
    }

    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    srv.perms
        .for_channel3(auth.user_id(), req.channel_id)
        .await?
        .ensure_view()?
        .check()?;

    let script = s
        .data()
        .script_get(req.script_id)
        .await?
        .ok_or(Error::NotFound)?;

    if script.channel_id != req.channel_id {
        return Err(Error::NotFound);
    }

    Ok((StatusCode::OK, Json(script)))
}

/// Script delete
#[handler(routes::script_delete)]
async fn script_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::script_delete::Request,
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
    let al = auth.audit_log(room_id);

    s.data().script_delete(req.script_id).await?;

    // TODO: remove script from service

    al.commit_success(AuditLogEntryType::ScriptDelete {
        channel_id: req.channel_id,
        script_id: req.script_id,
        changes: vec![], // TODO: populate changes
    })
    .await?;

    s.broadcast_room(
        room_id,
        auth.user.id,
        MessageSync::ScriptDelete {
            channel_id: req.channel_id,
            script_id: req.script_id,
        },
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Script content update
#[handler(routes::script_content_update)]
async fn script_content_update(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::script_content_update::Request,
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
    let _al = auth.audit_log(room_id);

    // TODO: validate that the script exists and belongs to this channel
    // TODO: copy some logic from script_create
    // TODO: make the script service reload the script

    Ok(Error::Unimplemented)
}

/// Script trigger
#[handler(routes::script_trigger)]
async fn script_trigger(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::script_trigger::Request,
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

    srv.perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .check()?;

    let run_ctl = srv
        .scripts
        .spawn(
            req.channel_id,
            req.script_id,
            ScriptInputData::Manual {
                id: "banana".to_owned(),
            },
        )
        .await?;
    let run = run_ctl.to_run();

    // TODO: create a run record in the database

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

/// Script version list
#[handler(routes::script_version_list)]
async fn script_version_list(
    auth: Auth3,
    State(s): State<Arc<ServerState>>,
    req: routes::script_version_list::Request,
) -> Result<impl IntoResponse> {
    if !s.config.scripts.enabled {
        return Err(Error::Unimplemented);
    }

    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    srv.perms
        .for_channel3(auth.user_id(), req.channel_id)
        .await?
        .ensure_view()?
        .check()?;

    let versions = s
        .data()
        .script_version_list_by_script(req.channel_id, req.script_id, req.pagination)
        .await?;

    Ok(Json(versions))
}

/// Script version get
#[handler(routes::script_version_get)]
async fn script_version_get(
    auth: Auth3,
    State(s): State<Arc<ServerState>>,
    req: routes::script_version_get::Request,
) -> Result<impl IntoResponse> {
    if !s.config.scripts.enabled {
        return Err(Error::Unimplemented);
    }

    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    srv.perms
        .for_channel3(auth.user_id(), req.channel_id)
        .await?
        .ensure_view()?
        .check()?;

    let version = s
        .data()
        .script_version_get(req.script_id, req.channel_id, req.version_id)
        .await?
        .ok_or(Error::NotFound)?;

    Ok((StatusCode::OK, Json(version)))
}

/// Script version delete
#[handler(routes::script_version_delete)]
async fn script_version_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::script_version_delete::Request,
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
    let al = auth.audit_log(room_id);

    // TODO: verify the version exists and belongs to this script

    s.data()
        .script_version_delete(req.script_id, req.version_id)
        .await?;

    al.commit_success(AuditLogEntryType::ScriptDelete {
        channel_id: req.channel_id,
        script_id: req.script_id,
        changes: Changes::new().build(), // TODO: add metadata to changes
    })
    .await?;

    s.broadcast_room(
        room_id,
        auth.user.id,
        MessageSync::ScriptVersionDelete {
            channel_id: req.channel_id,
            script_id: req.script_id,
            version_id: req.version_id,
        },
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Script version restore
#[handler(routes::script_version_restore)]
async fn script_version_restore(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::script_version_restore::Request,
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
    let _al = auth.audit_log(room_id);

    // TODO: do i soft-undelete the version (set deleted_at to None)? or do i create a new version with the same content as the old version?
    // TODO: verify the version exists and belongs to this script
    // TODO: make the script service reload the script

    Ok(Error::Unimplemented)
}

/// Script dependency graph
#[handler(routes::script_depends)]
async fn script_depends(
    auth: Auth3,
    State(s): State<Arc<ServerState>>,
    req: routes::script_depends::Request,
) -> Result<impl IntoResponse> {
    if !s.config.scripts.enabled {
        return Err(Error::Unimplemented);
    }

    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    srv.perms
        .for_channel3(auth.user_id(), req.channel_id)
        .await?
        .ensure_view()?
        .check()?;

    Ok(Error::Unimplemented)
}

/// Script dependency update
#[handler(routes::script_depends_update)]
async fn script_depends_update(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::script_depends_update::Request,
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
    let _al = auth.audit_log(room_id);

    Ok(Error::Unimplemented)
}

/// Script run list
#[handler(routes::script_run_list)]
async fn script_run_list(
    auth: Auth3,
    State(s): State<Arc<ServerState>>,
    req: routes::script_run_list::Request,
) -> Result<impl IntoResponse> {
    if !s.config.scripts.enabled {
        return Err(Error::Unimplemented);
    }

    auth.ensure_scopes(&[Scope::Full])?;

    let runs = s
        .data()
        .script_run_list(req.script_id, req.pagination)
        .await?;

    Ok(Json(runs))
}

/// Script run get
#[handler(routes::script_run_get)]
async fn script_run_get(
    auth: Auth3,
    State(s): State<Arc<ServerState>>,
    req: routes::script_run_get::Request,
) -> Result<impl IntoResponse> {
    if !s.config.scripts.enabled {
        return Err(Error::Unimplemented);
    }

    auth.ensure_scopes(&[Scope::Full])?;

    let run = s
        .data()
        .script_run_get(req.run_id)
        .await?
        .ok_or(Error::NotFound)?;

    Ok(Json(run))
}

/// Script run stop
#[handler(routes::script_run_stop)]
async fn script_run_stop(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    _req: routes::script_run_stop::Request,
) -> Result<impl IntoResponse> {
    if !s.config.scripts.enabled {
        return Err(Error::Unimplemented);
    }

    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;

    Ok(Error::Unimplemented)
}

/// Script run log
#[handler(routes::script_run_log)]
async fn script_run_log(
    auth: Auth3,
    State(s): State<Arc<ServerState>>,
    req: routes::script_run_log::Request,
) -> Result<impl IntoResponse> {
    if !s.config.scripts.enabled {
        return Err(Error::Unimplemented);
    }

    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    srv.perms
        .for_channel3(auth.user_id(), req.channel_id)
        .await?
        .ensure_view()?
        .check()?;

    let logs = s.data().script_log_list(req.run_id, req.pagination).await?;

    Ok(Json(logs))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(script_create))
        .routes(routes2!(script_list))
        .routes(routes2!(script_get))
        .routes(routes2!(script_delete))
        .routes(routes2!(script_content_update))
        .routes(routes2!(script_trigger))
        .routes(routes2!(script_version_list))
        .routes(routes2!(script_version_get))
        .routes(routes2!(script_version_delete))
        .routes(routes2!(script_version_restore))
        .routes(routes2!(script_depends))
        .routes(routes2!(script_depends_update))
        .routes(routes2!(script_run_list))
        .routes(routes2!(script_run_get))
        .routes(routes2!(script_run_stop))
        .routes(routes2!(script_run_log))
}
