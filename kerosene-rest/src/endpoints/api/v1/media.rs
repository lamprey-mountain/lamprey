use common::{
    v1::types::{
        AuditLogEntryType, MessageSync, Permission, SERVER_ROOM_ID, error::ErrorCode,
        federation::RemoteReq, misc::MediaIdReq, oauth::Scope,
    },
    v2::types::{
        MediaId,
        media::{MediaCreateSource, MediaCreated},
    },
};
use futures::StreamExt;
use lamprey_backend_services::services::{media::Import, search::SearchMediaVisibility};

use crate::prelude::*;

#[handler(routes::media_create)]
pub async fn create(
    req: Req<routes::media_create::Endpoint>,
) -> Result<routes::media_create::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let body = &req.inner().body;
    body.validate()
        .map_err(|err| ApiError::with_message(ErrorCode::InvalidData, err.to_string()))?;

    let globals = req.globals();
    let config = globals.config();
    if body.size().is_some_and(|size| size > config.media.max_size) {
        return Err(ApiError::from_code(ErrorCode::MediaTooBig).into());
    }

    let srv = req.services();
    let session_id = identity.session().map(|s| s.id);
    let media_id = MediaId::new();

    match &body.source {
        MediaCreateSource::Upload { size, .. } => {
            let mut import = Import::new_with_id(media_id, user.id).merge(body.clone());
            import.session_id = session_id;
            srv.media.import_from_upload(import).await.cast_internal()?;

            let upload_url = config
                .api_url
                .join(&format!("/api/v1/internal/media-upload/{media_id}"))
                .map_err(ServerError::internal)?;
            let media = MediaCreated {
                media_id,
                upload_url: Some(upload_url),
            };

            Ok(routes::media_create::Response {
                media,
                upload_offset: Some(0),
                upload_length: *size,
                content_length: None,
            })
        }
        MediaCreateSource::Download {
            size, source_url, ..
        } => {
            let mut import = Import::new_with_id(media_id, user.id).merge(body.clone());
            import.session_id = session_id;
            srv.media
                .import_from_url(import, source_url)
                .await
                .cast_internal()?;

            let media = MediaCreated {
                media_id,
                upload_url: None,
            };

            Ok(routes::media_create::Response {
                media,
                upload_offset: None,
                upload_length: None,
                content_length: *size,
            })
        }
    }
}

#[handler(routes::media_patch)]
pub async fn patch(
    req: Req<routes::media_patch::Endpoint>,
) -> Result<routes::media_patch::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let body = req.inner();
    body.body
        .validate()
        .map_err(|err| ApiError::with_message(ErrorCode::InvalidData, err.to_string()))?;

    let item = req
        .services()
        .media
        .patch(user.id, body.media_id, body.body.clone())
        .await
        .cast_internal()?;

    Ok(routes::media_patch::Response {
        media: (*item.media()).clone(),
    })
}

#[handler(routes::media_done)]
pub async fn done(req: Req<routes::media_done::Endpoint>) -> Result<routes::media_done::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let body = req.inner();
    let srv = req.services();
    let owned = match srv.media.upload_get(body.media_id).await {
        Some(upload) => upload.user_id() == user.id,
        None => srv
            .media
            .get(body.media_id)
            .await
            .is_ok_and(|item| item.media().user_id == Some(user.id)),
    };
    if !owned {
        return Err(ApiError::from_code(ErrorCode::UnknownMedia).into());
    }

    let mut item = srv.media.upload_done(body.media_id).await.cast_internal()?;
    let size = item.media().size;

    let media = if body.body.process_async {
        None
    } else {
        Some((*item.ready().await).clone())
    };
    let status = if body.body.process_async {
        http::StatusCode::ACCEPTED
    } else {
        http::StatusCode::OK
    };

    Ok(routes::media_done::Response {
        status,
        media,
        upload_offset: size,
        upload_length: size,
    })
}

#[handler(routes::media_get)]
pub async fn get(req: Req<routes::media_get::Endpoint>) -> Result<routes::media_get::Response> {
    let identity = req.identity();
    let srv = req.services();

    let media = match &req.inner().media_id {
        MediaIdReq::MediaId(media_id) => srv.media.get(*media_id).await.cast_internal()?.media(),
        MediaIdReq::MediaRemote(origin_id, hostname) => srv
            .federation
            .import_media(RemoteReq {
                origin_id: *origin_id,
                hostname: hostname.clone(),
            })
            .await
            .cast_internal()?,
    };

    if media.deleted_at.is_some() {
        let user = identity.ensure_user()?;
        let perms = srv
            .perms
            .for_room3(Some(user.id), SERVER_ROOM_ID)
            .await
            .cast_internal()?;
        if !perms.has(Permission::Admin) {
            return Err(ApiError::from_code(ErrorCode::UnknownMedia).into());
        }
    }

    Ok(routes::media_get::Response {
        media: (*media).clone(),
    })
}

#[handler(routes::media_quarantine)]
pub async fn quarantine(
    req: Req<routes::media_quarantine::Endpoint>,
) -> Result<routes::media_quarantine::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let globals = req.globals();
    let srv = req.services();
    let media_id = req.inner().media_id;

    srv.perms
        .for_room3(Some(user.id), SERVER_ROOM_ID)
        .await
        .cast_internal()?
        .ensure_view()?
        .needs(Permission::Admin)
        .check()?;

    let media = srv.media.get(media_id).await.cast_internal()?.media();
    if media.quarantine.is_some() {
        return Ok(routes::media_quarantine::Response {
            media: (*media).clone(),
        });
    }

    let media = srv
        .media
        .quarantine(media_id, req.headers().reason.clone())
        .await
        .cast_internal()?;

    req.audit_log()
        .push(
            SERVER_ROOM_ID,
            AuditLogEntryType::MediaQuarantine { media_id },
        )
        .success();

    globals
        .messaging()
        .broadcast_global(MessageSync::MediaUpdate {
            media: media.clone(),
        })
        .await
        .cast_internal()?;

    Ok(routes::media_quarantine::Response { media })
}

#[handler(routes::media_unquarantine)]
pub async fn unquarantine(
    req: Req<routes::media_unquarantine::Endpoint>,
) -> Result<routes::media_unquarantine::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let globals = req.globals();
    let srv = req.services();
    let media_id = req.inner().media_id;

    srv.perms
        .for_room3(Some(user.id), SERVER_ROOM_ID)
        .await
        .cast_internal()?
        .ensure_view()?
        .needs(Permission::Admin)
        .check()?;

    let media = srv.media.get(media_id).await.cast_internal()?.media();
    if media.quarantine.is_none() {
        return Ok(routes::media_unquarantine::Response {
            media: (*media).clone(),
        });
    }

    let media = srv.media.unquarantine(media_id).await.cast_internal()?;

    req.audit_log()
        .push(
            SERVER_ROOM_ID,
            AuditLogEntryType::MediaUnquarantine { media_id },
        )
        .success();

    globals
        .messaging()
        .broadcast_global(MessageSync::MediaUpdate {
            media: media.clone(),
        })
        .await
        .cast_internal()?;

    Ok(routes::media_unquarantine::Response { media })
}

#[handler(routes::media_delete)]
pub async fn delete(
    req: Req<routes::media_delete::Endpoint>,
) -> Result<routes::media_delete::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    req.services()
        .media
        .delete(user.id, req.inner().media_id)
        .await
        .cast_internal()?;

    Ok(routes::media_delete::Response {})
}

#[handler(routes::media_clone)]
pub async fn clone(
    req: Req<routes::media_clone::Endpoint>,
) -> Result<routes::media_clone::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    // TODO: implement srv.media.clone(MediaId)
    Err(ServerError::Unimplemented)
}

#[handler(routes::media_search)]
pub async fn search(
    req: Req<routes::media_search::Endpoint>,
) -> Result<routes::media_search::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();

    srv.perms
        .for_room3(Some(user.id), SERVER_ROOM_ID)
        .await
        .cast_internal()?
        .ensure_view()?
        .needs(Permission::Admin)
        .check()?;

    let results = srv
        .search
        .search_media(SearchMediaVisibility::Everything, req.inner().body.clone())
        .await
        .cast_internal()?;

    Ok(routes::media_search::Response { results })
}

#[handler(routes::media_upload)]
pub async fn upload(
    req: Req<routes::media_upload::Endpoint>,
) -> Result<routes::media_upload::Response> {
    let identity = req.identity();
    let user_id = identity.ensure_user()?.id;
    identity.ensure_scopes(&[Scope::Full])?;

    let media_id = req.inner().media_id;
    let upload_offset = req.inner().upload_offset;
    let srv = req.services();
    let body = req.into_inner().body;

    let mut upload = srv
        .media
        .upload_get(media_id)
        .await
        .ok_or_else(|| ApiError::from_code(ErrorCode::UnknownMedia))?;

    if upload.user_id() != user_id {
        return Err(ApiError::from_code(ErrorCode::UnknownMedia).into());
    }

    if upload.offset() != upload_offset {
        return Err(ApiError::with_message(
            ErrorCode::InvalidData,
            "upload offset does not match the current upload offset".to_string(),
        )
        .into());
    }

    let mut stream = body.into_data_stream();
    while let Some(chunk) = stream.next().await {
        upload
            .write(&chunk.map_err(ServerError::internal)?)
            .await
            .cast_internal()?;
    }

    let upload_offset = upload.offset();
    let upload_length = upload.expected_size();
    if !upload.expects_more() {
        drop(upload);
        srv.media.upload_done(media_id).await.cast_internal()?;
    }

    Ok(routes::media_upload::Response {
        upload_offset,
        upload_length,
    })
}

#[handler(routes::media_check)]
pub async fn check(
    req: Req<routes::media_check::Endpoint>,
) -> Result<routes::media_check::Response> {
    let user = req.identity().ensure_user()?;
    let media_id = req.inner().media_id;

    let srv = req.services();
    let upload = srv
        .media
        .upload_get(media_id)
        .await
        .ok_or_else(|| ApiError::from_code(ErrorCode::UnknownMedia))?;

    if upload.user_id() != user.id {
        return Err(ApiError::from_code(ErrorCode::UnknownMedia).into());
    }

    Ok(routes::media_check::Response {
        upload_offset: upload.offset(),
        upload_length: upload.expected_size(),
    })
}

export_routes!(
    create,
    patch,
    done,
    get,
    quarantine,
    unquarantine,
    delete,
    clone,
    search,
    upload,
    check,
);
