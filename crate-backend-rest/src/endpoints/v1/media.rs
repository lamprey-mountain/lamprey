use axum::http::HeaderMap;
use common::{
    v1::types::oauth::Scope,
    v2::types::{
        MediaId,
        media::{MediaCreateSource, MediaCreated},
    },
};

use crate::prelude::*;

#[axum::debug_handler]
#[handler(routes::media_create_new)]
async fn create(
    req: Req<routes::media_create_new::Endpoint>,
) -> Result<routes::media_create_new::Response> {
    let user = req.auth.ensure_user()?;
    user.ensure_unsuspended()?;
    req.auth.ensure_scopes(&[Scope::Full])?;
    req.body.create.validate()?;

    let srv = req.globals.services();
    let config = req.globals.config();
    let json = req.body.create;
    match &json.source {
        MediaCreateSource::Upload { size, .. } => {
            if *size > Some(config.media.max_size) {
                return Err(Error::TooBig);
            }

            let media_id = MediaId::new();
            // TODO: actually import media
            // let import = Import::new_with_id(media_id, user.id).merge(json.clone());
            // srv.media.import_from_upload(import).await?;
            let upload_url = Some(
                config
                    .api_url
                    .join(&format!("/api/v1/internal/media-upload/{media_id}"))?,
            );
            let created = MediaCreated {
                media_id,
                upload_url,
            };

            Ok(routes::media_create_new::Response {
                created,
                upload_offset: Some(0),
                content_length: *size,
            })
        }
        MediaCreateSource::Download {
            size, source_url, ..
        } => {
            if size.is_some_and(|sz| sz > config.media.max_size) {
                return Err(Error::TooBig);
            }

            let media_id = MediaId::new();
            // TODO: actually import media
            // let import = Import::new_with_id(media_id, user.id).merge(json.clone());
            // srv.media.import_from_url(import, source_url).await?;
            let created = MediaCreated {
                media_id,
                upload_url: None,
            };
            Ok(routes::media_create_new::Response {
                created,
                upload_offset: None,
                content_length: *size,
            })
        }
    }
}

#[handler(routes::media_upload)]
#[axum::debug_handler]
async fn upload(
    req: Req<routes::media_upload::Endpoint>,
) -> Result<routes::media_upload::Response> {
    req.auth.ensure_scopes(&[Scope::Full])?;

    // TODO: implement

    Ok(routes::media_upload::Response {
        upload_offset: todo!(),
        content_length: todo!(),
    })
}
