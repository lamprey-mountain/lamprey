use common::{
    v1::types::{
        Embed, MessageSync, UserWithRelationship,
        ack::{AckBulkItem, AckType},
        oauth::Scope,
        unfurl::{UnfurlerDebugResponse, UnfurlerResponse},
        util::Time,
    },
    v2::types::{
        MessageId,
        media::{MediaCreate, MediaCreateSource},
    },
};
use futures::stream::{FuturesOrdered, FuturesUnordered, StreamExt};
use http::StatusCode;
use lamprey_backend_services::{
    compat::types::UserIdReq,
    services::{media::Import, messages::create2::Create},
};
use tracing::warn;

use crate::prelude::*;

// TODO: lamprey_unfurl should probably not be used here directly. logic should be moved to the embeds service.

#[handler(routes::unfurler_debug)]
pub async fn debug(
    req: Req<routes::unfurler_debug::Endpoint>,
) -> Result<routes::unfurler_debug::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Full])?;
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;

    let srv = req.services();
    let (generations, log) = srv
        .embed
        .unfurl_with_logs(&req.inner().body.url)
        .await
        .cast_internal()?;

    let mut embeds = FuturesUnordered::new();

    for mut g in generations {
        let srv = srv.clone();
        let user_id = user.id;
        embeds.push(async move {
            let mut media = FuturesUnordered::new();
            for pending in g.pending_media() {
                let srv = srv.clone();
                media.push(async move {
                    let import = Import::new(user_id).merge(MediaCreate {
                        alt: pending.alt,
                        strip_exif: false,
                        source: MediaCreateSource::Download {
                            filename: None,
                            size: None,
                            source_url: pending.url.clone(),
                        },
                    });
                    let mut item = srv
                        .media
                        .import_from_url(import, &pending.url)
                        .await
                        .cast_internal()?;
                    Result::Ok((pending.placeholder_media_id, item.ready().await))
                });
            }

            while let Some(result) = media.next().await {
                if let Ok((pid, media)) = result {
                    g.update_media(
                        pid,
                        lamprey_unfurl::util::EmbedMedia::Finished((*media).clone()),
                    );
                }
            }

            Result::Ok(g.into_embed())
        });
    }

    Ok(routes::unfurler_debug::Response {
        body: UnfurlerDebugResponse {
            log,
            embeds: embeds
                .collect::<Vec<_>>()
                .await
                .into_iter()
                .collect::<Result<Vec<_>>>()?,
        },
    })
}

#[handler(routes::unfurler_unfurl)]
pub async fn unfurl(
    req: Req<routes::unfurler_unfurl::Endpoint>,
) -> Result<routes::unfurler_unfurl::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Full])?;
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;

    req.inner()
        .body
        .validate()
        .map_err(lamprey_backend_core::Error::Validation)
        .cast_internal()?;

    let srv = req.services();
    let mut tasks = FuturesUnordered::new();

    for url in &req.inner().body.urls {
        let srv = srv.clone();
        let user_id = user.id;
        let url = url.clone();
        tasks.push(async move {
            let mut embeds = FuturesUnordered::new();

            let Ok(generations) = srv.embed.unfurl(&url).await else {
                return vec![];
            };

            for mut g in generations {
                let srv = srv.clone();
                embeds.push(async move {
                    let mut media = FuturesUnordered::new();
                    for pending in g.pending_media() {
                        let srv = srv.clone();
                        media.push(async move {
                            let import = Import::new(user_id).merge(MediaCreate {
                                alt: pending.alt,
                                strip_exif: false,
                                source: MediaCreateSource::Download {
                                    filename: None,
                                    size: None,
                                    source_url: pending.url.clone(),
                                },
                            });
                            let mut item = srv
                                .media
                                .import_from_url(import, &pending.url)
                                .await
                                .cast_internal()?;
                            Result::Ok((pending.placeholder_media_id, item.ready().await))
                        });
                    }

                    while let Some(result) = media.next().await {
                        if let Ok((pid, media)) = result {
                            g.update_media(
                                pid,
                                lamprey_unfurl::util::EmbedMedia::Finished((*media).clone()),
                            );
                        }
                    }

                    Result::Ok(g.into_embed())
                });
            }

            embeds
                .collect::<Vec<_>>()
                .await
                .into_iter()
                .filter_map(|r| r.ok())
                .collect::<Vec<Embed>>()
        });
    }

    let embeds: Vec<Vec<Embed>> = tasks.collect().await;

    Ok(routes::unfurler_unfurl::Response {
        body: UnfurlerResponse {
            embeds: embeds.into_iter().flatten().collect(),
        },
    })
}

export_routes!(debug, unfurl);
