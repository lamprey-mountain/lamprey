use axum::http::HeaderMap;
use common::{
    v1::types::{
        error::{ErrorField, ErrorFieldType},
        oauth::Scope,
    },
    v2::types::{
        MediaId,
        media::{MediaCreateSource, MediaCreated},
    },
};
use kerosene_core::error::ErrorCode;

use crate::prelude::*;

#[handler(routes::media_create)]
async fn create(
    req: Req<routes::media_create::Endpoint>,
) -> Result<routes::media_create::Response> {
    let user = req.identity().ensure_user()?;
    user.ensure_unsuspended()?;
    req.identity().ensure_scopes(&[Scope::Full])?;
    // req.inner().body.validate()?;

    // let srv = req.services();
    // let config = req.globals().config();
    // let json = req.inner().body.clone();

    // if json.size().is_some_and(|sz| sz > config.media.max_size) {
    //     return Err(ApiError {
    //         fields: vec![ErrorField {
    //             key: vec!["size".into()],
    //             message: "media is too big".into(),
    //             ty: ErrorFieldType::Range {
    //                 min: None,
    //                 max: Some(config.media.max_size),
    //             },
    //         }],
    //         ..ApiError::from_code(ErrorCode::MediaTooBig)
    //     });
    // }

    // let media_id = MediaId::new();
    // match &json.source {
    //     MediaCreateSource::Upload { size, .. } => {
    //         // TODO: actually import media
    //         // let import = Import::new_with_id(media_id, user.id).merge(json.clone());
    //         // srv.media.import_from_upload(import).await?;
    //         let upload_url = Some(
    //             config
    //                 .api_url
    //                 .join(&format!("/api/v1/internal/media-upload/{media_id}"))?,
    //         );
    //         let created = MediaCreated {
    //             media_id,
    //             upload_url,
    //         };

    //         Ok(routes::media_create::Response {
    //             created,
    //             upload_offset: Some(0),
    //             content_length: *size,
    //         })
    //     }
    //     MediaCreateSource::Download {
    //         size, source_url, ..
    //     } => {
    //         // TODO: actually import media
    //         // let import = Import::new_with_id(media_id, user.id).merge(json.clone());
    //         // srv.media.import_from_url(import, source_url).await?;
    //         let created = MediaCreated {
    //             media_id,
    //             upload_url: None,
    //         };
    //         Ok(routes::media_create::Response {
    //             created,
    //             upload_offset: None,
    //             content_length: *size,
    //         })
    //     }
    // }

    todo!()
}

#[handler(routes::media_upload)]
async fn upload(
    req: Req<routes::media_upload::Endpoint>,
) -> Result<routes::media_upload::Response> {
    // req.auth.ensure_scopes(&[Scope::Full])?;

    // TODO: implement

    Ok(routes::media_upload::Response {
        upload_offset: todo!(),
        content_length: todo!(),
    })
}
