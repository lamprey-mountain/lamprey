use common::{
    v1::types::{
        MessageSync, UserWithRelationship,
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
use futures::stream::{FuturesOrdered, FuturesUnordered};
use http::StatusCode;
use lamprey_backend_services::{
    compat::types::UserIdReq,
    services::{media::Import, messages::create2::Create},
};
use tracing::warn;

use crate::prelude::*;

#[handler(routes::unfurler_debug)]
pub async fn debug(
    req: Req<routes::unfurler_debug::Endpoint>,
) -> Result<routes::unfurler_debug::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Full])?;
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;

    let srv = req.services();
    let (generations, log) = srv.embed.unfurl_with_logs(&req.inner().body.url).await?;

    let mut embeds = FuturesUnordered::new();

    for mut g in generations {
        tasks.push(async {
            let mut media = FuturesUnordered::new();
            for pending in g.pending_media() {
                media.push(async {
                    let import = Import::new(user.id).merge(MediaCreate {
                        alt: pending.alt,
                        strip_exif: false,
                        source: MediaCreateSource::Download {
                            filename: None,
                            size: None,
                            source_url: pending.url.clone(),
                        },
                    });
                    let mut item = srv.media.import_from_url(import, &pending.url).await?;
                    (pending.placeholder_media_id, item.ready().await)
                });
            }

            while let Some((pid, media)) = media.next() {
                g.update_media(
                    pid,
                    lamprey_unfurl::util::EmbedMedia::Finished((*media).clone()),
                );
            }

            Result::Ok(g.into_embed())
        });
    }

    Ok(routes::unfurler_debug::Response {
        body: UnfurlerDebugResponse {
            log,
            embeds: embeds.await,
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

    req.inner().body.validate()?;

    let srv = req.services();
    let mut tasks = FuturesUnordered::new();

    for url in &req.inner().body.urls {
        tasks.push(async {
            let generations = srv.embed.unfurl(url).await.cast_internal()?;
            let mut embeds = FuturesUnordered::new();

            for mut g in generations {
                tasks.push(async {
                    let mut media = FuturesUnordered::new();
                    for pending in g.pending_media() {
                        media.push(async {
                            let import = Import::new(user.id).merge(MediaCreate {
                                alt: pending.alt,
                                strip_exif: false,
                                source: MediaCreateSource::Download {
                                    filename: None,
                                    size: None,
                                    source_url: pending.url.clone(),
                                },
                            });
                            let mut item = srv.media.import_from_url(import, &pending.url).await?;
                            (pending.placeholder_media_id, item.ready().await)
                        });
                    }

                    while let Some((pid, media)) = media.next() {
                        g.update_media(
                            pid,
                            lamprey_unfurl::util::EmbedMedia::Finished((*media).clone()),
                        );
                    }

                    Result::Ok(g.into_embed())
                });
            }

            Result::Ok(embeds.await)
        });
    }

    Ok(routes::unfurler_unfurl::Response {
        body: UnfurlerResponse {
            embeds: tasks.await.into_iter().flatten().collect(),
        },
    })
}

export_routes!(debug, unfurl);
