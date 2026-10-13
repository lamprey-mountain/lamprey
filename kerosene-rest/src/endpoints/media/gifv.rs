use crate::{
    endpoints::media::util::{MediaInfo, calculate_response_metadata},
    prelude::*,
};
use common::util::body::Body;
use kerosene_core::types::media::MediaPaths;
use routes::media_proxy as routes;

#[handler(routes::gifv_get)]
async fn get(req: Req<routes::gifv_get::Endpoint>) -> Result<routes::gifv_get::Response> {
    let globals = req.globals();
    let srv = req.services();
    let media_id = req.inner().media_id;
    let query = &req.inner().query;

    let mut media_item = srv.media.get(media_id).await.cast_internal()?;
    let media = media_item.ready().await;

    // only support transcoding gifs for now
    if media
        .content_type
        .parse()
        .map_err(Error::internal)?
        .essence()
        .to_string()
        != "image/gif"
    {
        return Err(ApiError::from_code(ErrorCode::InvalidData).into());
    }

    let paths = MediaPaths::new("media/");
    let gifv_path = paths.gifv(media.id);

    media_item.generate_gifv().await.cast_internal()?;

    let meta = calculate_response_metadata(
        req.headers(),
        &MediaInfo::Gifv {
            media: &media,
            content_length: None,
        },
    )?;

    if meta.unmodified {
        Ok(routes::gifv_get::Response {
            status: meta.status(),
            headers: meta.headers.into(),
            body: Body::empty(),
        })
    } else {
        let meta_data = globals
            .blobs()
            .stat(&gifv_path)
            .await
            .map_err(|_| ApiError::from_code(ErrorCode::NotFound))?;
        let content_length = meta_data.content_length();

        let meta = calculate_response_metadata(
            req.headers(),
            &MediaInfo::Gifv {
                media: &media,
                content_length: Some(content_length),
            },
        )?;

        let body = if req.method() == http::Method::HEAD {
            Body::empty()
        } else {
            let reader = globals
                .blobs()
                .reader(&gifv_path)
                .await
                .map_err(|err| ServerError::Internal(Box::new(err)))?;

            let stream = if let Some(range) = meta.range {
                reader
                    .into_bytes_stream(range)
                    .await
                    .map_err(|err| ServerError::Internal(Box::new(err)))?
            } else {
                reader
                    .into_bytes_stream(..)
                    .await
                    .map_err(|err| ServerError::Internal(Box::new(err)))?
            };

            Body::from_stream(stream)
        };

        Ok(routes::gifv_get::Response {
            status: meta.status(),
            headers: meta.headers.into(),
            body,
        })
    }
}

export_routes!(get);
