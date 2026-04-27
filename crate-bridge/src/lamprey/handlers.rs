//! Lamprey actor message handlers

use anyhow::Result;
use common::v1::types::util::Time;
use common::v1::types::MediaId;
use common::v1::types::{self, misc::UserIdReq, pagination::PaginationQuery, RoomMemberPut};
use common::v2::types::media::{Media, MediaCreate, MediaCreateSource};
use dashmap::DashMap;
use sdk::Http;
use std::sync::Arc;
use tokio::sync::oneshot;
use tracing::{debug, info};

use crate::bridge_common::Globals;
use crate::lamprey::messages::{LampreyMessage, LampreyResponse};

type TokioHashMap = DashMap<MediaId, oneshot::Sender<Media>>;

pub(super) async fn handle_lamprey_message(
    http: &Http,
    globals: Arc<Globals>,
    media_processed: Arc<TokioHashMap>,
    msg: LampreyMessage,
) -> Result<LampreyResponse> {
    match msg {
        LampreyMessage::MediaUpload {
            filename,
            bytes,
            user_id,
        } => {
            let req = MediaCreate {
                strip_exif: false,
                alt: None,
                source: MediaCreateSource::Upload {
                    filename,
                    size: Some(bytes.len() as u64),
                },
            };
            let upload = http.for_puppet(user_id).media_create(&req).await?;

            let upload_id = upload.media_id;
            let (tx, rx) = oneshot::channel();
            media_processed.insert(upload_id, tx);

            http.for_puppet(user_id)
                .media_upload(&upload, bytes)
                .await?;

            // HACK: media_done seems to be broken currently, so i have to do some channel gymnastics to get this to work :(
            // let media = http
            //     .for_puppet(user_id)
            //     .media_done(
            //         upload.media_id,
            //         &MediaDoneParams {
            //             process_async: true,
            //         },
            //     )
            //     .await?;

            let media = match rx.await {
                Ok(media) => media,
                Err(_) => {
                    return Err(anyhow::anyhow!(
                        "media processed handler dropped the channel"
                    ));
                }
            };

            Ok(LampreyResponse::Media(media))
        }
        LampreyMessage::MessageGet {
            thread_id,
            message_id,
        } => http
            .message_get(thread_id, message_id)
            .await
            .map(LampreyResponse::Message),
        LampreyMessage::MessageList { thread_id, query } => http
            .message_list(thread_id, &query)
            .await
            .map(LampreyResponse::MessageList),
        LampreyMessage::MessageCreate {
            thread_id,
            user_id,
            req,
        } => {
            let timestamp = Time::now_utc();

            http.for_puppet(user_id)
                .message_create_with_timestamp(thread_id, &req, timestamp)
                .await
                .map(LampreyResponse::Message)
        }
        LampreyMessage::MessageCreateWithTimestamp {
            thread_id,
            user_id,
            req,
            timestamp,
        } => http
            .for_puppet(user_id)
            .message_create_with_timestamp(thread_id, &req, timestamp)
            .await
            .map(LampreyResponse::Message),
        LampreyMessage::MessageUpdate {
            thread_id,
            message_id,
            user_id,
            req,
        } => http
            .for_puppet(user_id)
            .message_edit(thread_id, message_id, &req)
            .await
            .map(LampreyResponse::Message),
        LampreyMessage::MessageDelete {
            thread_id,
            message_id,
            user_id,
        } => http
            .for_puppet(user_id)
            .message_delete(thread_id, message_id)
            .await
            .map(|_| LampreyResponse::Empty),
        LampreyMessage::MessageReact {
            thread_id,
            message_id,
            user_id,
            reaction,
        } => http
            .for_puppet(user_id)
            .message_react(thread_id, message_id, reaction)
            .await
            .map(|_| LampreyResponse::Empty),
        LampreyMessage::MessageUnreact {
            thread_id,
            message_id,
            user_id,
            reaction,
        } => http
            .for_puppet(user_id)
            .message_unreact(thread_id, message_id, reaction)
            .await
            .map(|_| LampreyResponse::Empty),
        LampreyMessage::TypingStart { thread_id, user_id } => http
            .for_puppet(user_id)
            .channel_typing(thread_id)
            .await
            .map(|_| LampreyResponse::Empty),
        LampreyMessage::PuppetEnsure {
            name,
            key,
            room_id,
            bot,
        } => {
            let app_id = globals.config.lamprey_application_id;
            let user = http
                .puppet_ensure(
                    app_id,
                    key,
                    &types::PuppetCreate {
                        name,
                        description: None,
                        bot,
                        system: false,
                    },
                )
                .await?;
            debug!("ensured user");

            http.room_member_add(
                room_id,
                UserIdReq::UserId(user.id),
                &RoomMemberPut::default(),
            )
            .await?;
            debug!("ensured room member");
            Ok(LampreyResponse::User(user))
        }
        LampreyMessage::UserFetch { user_id } => http
            .user_get(UserIdReq::UserId(user_id))
            .await
            .map(|res| LampreyResponse::User(res.inner)),
        LampreyMessage::UserUpdate { user_id, patch } => match http
            .for_puppet(user_id)
            .user_update(UserIdReq::UserId(user_id), &patch)
            .await
        {
            Ok(user) => Ok(LampreyResponse::User(user)),
            Err(e) => {
                tracing::warn!("failed to update user {user_id}: {e}");
                Ok(LampreyResponse::Empty)
            }
        },
        LampreyMessage::UserSetPresence { user_id, patch } => http
            .for_puppet(user_id)
            .user_set_presence(UserIdReq::UserId(user_id), &patch)
            .await
            .map(|_| LampreyResponse::Empty),
        LampreyMessage::RoomMemberPatch {
            room_id,
            user_id,
            patch,
        } => http
            .room_member_patch(room_id, UserIdReq::UserId(user_id), &patch)
            .await
            .map(LampreyResponse::RoomMember),
        LampreyMessage::RoomThreads { room_id } => {
            let mut all_threads = Vec::new();
            let mut query = PaginationQuery::default();
            loop {
                info!("get room threads");
                let res = http.channel_list(room_id, &query).await?;
                debug!("threads: {res:?}");
                all_threads.extend(res.items);
                if let Some(cursor) = res.cursor {
                    query.from = Some(cursor.parse()?);
                } else {
                    break;
                }
                if !res.has_more {
                    break;
                }
            }
            Ok(LampreyResponse::RoomThreads(all_threads))
        }
        LampreyMessage::CreateThread {
            room_id,
            name,
            topic,
            ty,
            parent_id,
        } => {
            let res = http
                .channel_create_room(
                    room_id,
                    &types::ChannelCreate {
                        name,
                        description: topic,
                        ty,
                        parent_id,
                        ..Default::default()
                    },
                )
                .await?;
            Ok(LampreyResponse::Channel(res))
        }
    }
}
