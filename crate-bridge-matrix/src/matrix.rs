use std::sync::Arc;

use anyhow::Result;
use matrix_sdk::{
    attachment::AttachmentConfig,
    config::SyncSettings,
    deserialized_responses::TimelineEventKind,
    ruma::{
        events::room::message::{
            OriginalRoomMessageEvent, OriginalSyncRoomMessageEvent, RoomMessageEventContent,
        },
        OwnedEventId, OwnedRoomId,
    },
    Client, Room,
};
use tokio::sync::{mpsc, oneshot};
use tracing::error;

use crate::common::Globals;

/// matrix actor
pub struct Matrix {
    globals: Arc<Globals>,
    recv: mpsc::Receiver<MatrixMessage>,
}

/// matrix actor message
#[allow(clippy::enum_variant_names)]
pub enum MatrixMessage {
    Send {
        room_id: OwnedRoomId,
        payload: RoomMessageEventContent,
        response: oneshot::Sender<OwnedEventId>,
    },
    SendAttachment {
        room_id: OwnedRoomId,
        payload: types::Media,
        response: oneshot::Sender<OwnedEventId>,
    },
    Read {
        room_id: OwnedRoomId,
        event_id: OwnedEventId,
        response: oneshot::Sender<OriginalRoomMessageEvent>,
    },
}

impl Matrix {
    pub fn new(globals: Arc<Globals>, recv: mpsc::Receiver<MatrixMessage>) -> Matrix {
        Self { globals, recv }
    }

    pub async fn connect(mut self) -> Result<()> {
        let token = std::env::var("MATRIX_TOKEN").expect("missing MATRIX_TOKEN");
        let base_url = std::env::var("MATRIX_BASE_URL").expect("missing MATRIX_BASE_URL");
        let client = matrix_sdk::Client::builder()
            .homeserver_url(base_url)
            .build()
            .await?;
        client.matrix_auth().login_token(&token).await?;

        client.add_event_handler(|ev: OriginalSyncRoomMessageEvent, room: Room| async move {
            // match ev.content.msgtype {
            //     MessageType::Text(text) => {
            //         self.globals.portal_send_mx(
            //             channel_id,
            //             PortalMessage::Matrix {
            //                 message_id: deleted_message_id,
            //             },
            //         );
            //         text.body
            //     },
            //     // MessageType::Audio(audio_message_event_content) => todo!(),
            //     // MessageType::Emote(emote_message_event_content) => todo!(),
            //     // MessageType::File(file_message_event_content) => todo!(),
            //     // MessageType::Image(image_message_event_content) => todo!(),
            //     // MessageType::Location(location_message_event_content) => todo!(),
            //     // MessageType::Notice(notice_message_event_content) => todo!(),
            //     // MessageType::ServerNotice(server_notice_message_event_content) => todo!(),
            //     // MessageType::Video(video_message_event_content) => todo!(),
            //     // MessageType::VerificationRequest(key_verification_request_event_content) => todo!(),
            //     _ => todo!(),
            // }
            // RoomMessageEventContent::text_markdown().make_reply_to(, , )
            // client.get_room(todo!()).unwrap().send()
        });

        let c = client.clone();
        tokio::spawn(async move {
            while let Some(msg) = self.recv.recv().await {
                match handle(msg, &c).await {
                    Ok(_) => {}
                    Err(err) => error!("{err}"),
                };
            }
        });

        client.sync(SyncSettings::default()).await?;

        Ok(())
    }
}

async fn handle(msg: MatrixMessage, mx: &Client) -> Result<()> {
    match msg {
        MatrixMessage::Send {
            room_id,
            payload,
            response,
        } => {
            let event_id = mx.get_room(&room_id).unwrap().send(payload).await?.event_id;
            let _ = response.send(event_id);
        }
        MatrixMessage::SendAttachment {
            room_id,
            payload,
            response,
        } => {
            let bytes = reqwest::get(payload.url)
                .await?
                .error_for_status()?
                .bytes()
                .await?;
            let event_id = mx
                .get_room(&room_id)
                .unwrap()
                .send_attachment(
                    &payload.filename,
                    &payload.mime.parse()?,
                    bytes.into(),
                    AttachmentConfig::default().caption(payload.alt),
                )
                .await?
                .event_id;
            let _ = response.send(event_id);
        }
        MatrixMessage::Read {
            room_id,
            event_id,
            response,
        } => {
            let ev = mx
                .get_room(&room_id)
                .unwrap()
                .event(&event_id, None)
                .await?;
            let m: OriginalRoomMessageEvent = match ev.kind {
                TimelineEventKind::UnableToDecrypt { event, utd_info } => todo!(),
                TimelineEventKind::Decrypted(ev) => todo!(),
                TimelineEventKind::PlainText { event } => event.deserialize_as()?,
            };
            let _ = response.send(m);
        }
    }
    //     DiscordMessage::WebhookExecute {
    //         url,
    //         payload,
    //         response,
    //     } => {
    //         let hook = self.get_hook(url, http).await?;
    //         let msg = hook
    //             .execute(&http, true, payload)
    //             .await?
    //             .expect("wait should return message");
    //         response.send(msg).unwrap();
    //     }
    //     DiscordMessage::WebhookMessageEdit {
    //         url,
    //         message_id,
    //         payload,
    //         response,
    //     } => {
    //         let hook = self.get_hook(url, http).await?;
    //         let msg = hook.edit_message(&http, message_id, payload).await?;
    //         response.send(msg).unwrap();
    //     }
    //     DiscordMessage::WebhookMessageDelete {
    //         url,
    //         thread_id,
    //         message_id,
    //         response,
    //     } => {
    //         let hook = self.get_hook(url, http).await?;
    //         hook.delete_message(&http, thread_id, message_id).await?;
    //         response.send(()).unwrap();
    //     }
    // }
    Ok(())
}
