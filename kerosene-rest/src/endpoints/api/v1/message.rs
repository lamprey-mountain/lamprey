use common::{
    v1::types::{
        MessageSync,
        ack::{AckBulkItem, AckType},
        oauth::Scope,
        util::Time,
    },
    v2::types::MessageId,
};
use http::StatusCode;
use tracing::warn;

use crate::prelude::*;

#[handler(routes::message_create)]
pub async fn create(
    req: Req<routes::message_create::Endpoint>,
) -> Result<routes::message_create::Response> {
    let identity = req.identity();
    let session = identity.ensure_session()?; // NOTE: make optional?
    let user = identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;
    user.ensure_unsuspended()?;

    // grab these before into_innering req
    let globals = req.globals();
    let srv = req.services();
    let session_id = session.id;
    let user_id = user.id;

    let body = req.into_inner();
    let channel_id = body.channel_id;

    let timestamp = body.timestamp.and_then(|secs| {
        time::OffsetDateTime::from_unix_timestamp(secs)
            .ok()
            .map(Time::from)
    });

    let message_id = MessageId::new();
    let message = srv
        .messages
        .create3(channel_id, user_id)
        .id(message_id)
        .session(Some(session_id))
        .timestamp(timestamp)
        .nonce(body.idempotency_key)
        .create_default(body.message)
        .await
        .cast_internal()?;

    // return 201 if message was created and 200 if message already existed
    let status = if message.id == message_id {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    let message_id = message.id;

    // TODO: move this logic to notifications or ack service
    // automatically ack the channel for the user who sent the message
    let mut txn = globals.begin().await.cast_internal()?;
    txn.unread_ack_bulk(
        user_id,
        &[AckBulkItem {
            ty: AckType::Message {
                channel_id,
                message_id,
                mention_count: 0,
            },
        }],
    )
    .await
    .cast_internal()?;
    txn.commit().await.cast_internal()?;

    srv.channels.invalidate_user(channel_id, user_id).await;

    Ok(routes::message_create::Response { message, status })
}

#[handler(routes::message_get)]
pub async fn get(req: Req<routes::message_get::Endpoint>) -> Result<routes::message_get::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let user_id = identity.user_id();
    let body = req.inner();

    srv.perms
        .for_channel3(user_id, body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .check()?;

    let message = srv
        .messages
        .get(body.channel_id, body.message_id, user_id)
        .await
        .cast_internal()?;

    Ok(routes::message_get::Response { message })
}

export_routes!(create, get);
