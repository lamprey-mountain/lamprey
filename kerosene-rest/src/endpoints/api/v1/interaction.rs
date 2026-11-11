use common::v1::types::misc::InteractionMessageReq;
use common::v1::types::{MessageSync, Permission, UserId, oauth::Scope, util::Time};

use crate::{prelude::*, util::request::ReqAuth};

#[handler(routes::interaction_create)]
pub async fn create(
    req: Req<routes::interaction_create::Endpoint>,
) -> Result<routes::interaction_create::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    if let Some(channel_id) = body.create.ty.channel_id() {
        let mut perms = srv
            .perms
            .for_channel3(Some(user.id), channel_id)
            .await
            .cast_internal()?
            .ensure_view()?;
        perms
            .needs_unlocked()
            .needs(Permission::MessageCreate)
            .check()?;
    } else {
        tracing::warn!("not enforcing any permissions for interaction not in a channel");
    }

    let interaction = srv
        .interactions
        .create(user.id, body.idempotency_key.clone(), body.create.clone())
        .await
        .cast_internal()?;

    Ok(routes::interaction_create::Response { interaction })
}

#[handler(routes::interaction_respond)]
pub async fn respond(
    req: Req<routes::interaction_respond::Endpoint>,
) -> Result<routes::interaction_respond::Response> {
    let body = req.inner();

    req.services()
        .interactions
        .handle_callback(
            body.interaction_id,
            body.token.clone(),
            body.response.clone(),
        )
        .await
        .cast_internal()?;

    Ok(routes::interaction_respond::Response {})
}

#[handler(routes::interaction_message_create)]
pub async fn message_create(
    req: Req<routes::interaction_message_create::Endpoint>,
) -> Result<routes::interaction_message_create::Response> {
    let identity = req.identity();
    identity.ensure_user()?;
    let body = req.inner();

    let mut auth = ReqAuth::new(&req);
    let message = req
        .services()
        .interactions
        .handle_followup_message_create(
            &mut auth,
            body.interaction_id,
            body.token.clone(),
            body.message.clone(),
            body.idempotency_key.clone(),
        )
        .await
        .cast_internal()?;

    Ok(routes::interaction_message_create::Response { message })
}

#[handler(routes::interaction_message_get)]
pub async fn message_get(
    req: Req<routes::interaction_message_get::Endpoint>,
) -> Result<routes::interaction_message_get::Response> {
    let body = req.inner();

    let srv = req.services();
    let inter = srv.interactions.get(body.interaction_id).cast_internal()?;

    if inter.inner().token.as_ref() != Some(&body.token) {
        return Err(ApiError::with_message(
            ErrorCode::InvalidData,
            "unknown or expired interaction".to_string(),
        )
        .into());
    }

    let channel_id = inter.inner().ty.channel_id().ok_or_else(|| {
        ApiError::with_message(
            ErrorCode::InvalidData,
            "interaction type has no channel id".to_string(),
        )
    })?;

    let message_id = match body.message_id {
        InteractionMessageReq::MessageOriginal => inter.original_message_id().ok_or_else(|| {
            ApiError::with_message(
                ErrorCode::InvalidData,
                "interaction has no original message".to_string(),
            )
        })?,
        InteractionMessageReq::MessageId(id) => id,
    };

    let application_user_id: UserId = (*inter.inner().application_id).into();
    let message = srv
        .messages
        .get(channel_id, message_id, Some(application_user_id))
        .await
        .cast_internal()?;

    if message.author_id != application_user_id {
        return Err(ApiError::from_code(ErrorCode::UnknownMessage).into());
    }

    Ok(routes::interaction_message_get::Response { message })
}

#[handler(routes::interaction_message_edit)]
pub async fn message_edit(
    req: Req<routes::interaction_message_edit::Endpoint>,
) -> Result<routes::interaction_message_edit::Response> {
    let body = req.inner();

    let srv = req.services();
    let inter = srv.interactions.get(body.interaction_id).cast_internal()?;

    if inter.inner().token.as_ref() != Some(&body.token) {
        return Err(ApiError::with_message(
            ErrorCode::InvalidData,
            "unknown or expired interaction".to_string(),
        )
        .into());
    }

    let channel_id = inter.inner().ty.channel_id().ok_or_else(|| {
        ApiError::with_message(
            ErrorCode::InvalidData,
            "interaction type has no channel id".to_string(),
        )
    })?;

    let message_id = match body.message_id {
        InteractionMessageReq::MessageOriginal => inter.original_message_id().ok_or_else(|| {
            ApiError::with_message(
                ErrorCode::InvalidData,
                "interaction has no original message".to_string(),
            )
        })?,
        InteractionMessageReq::MessageId(id) => id,
    };

    let application_user_id: UserId = (*inter.inner().application_id).into();
    let header_timestamp = body.timestamp.and_then(|secs| {
        time::OffsetDateTime::from_unix_timestamp(secs)
            .ok()
            .map(Time::from)
    });

    let (_status, message) = srv
        .messages
        .edit_as_application(
            channel_id,
            message_id,
            application_user_id,
            body.patch.clone(),
            header_timestamp,
        )
        .await
        .cast_internal()?;

    Ok(routes::interaction_message_edit::Response { message })
}

#[handler(routes::interaction_message_delete)]
pub async fn message_delete(
    req: Req<routes::interaction_message_delete::Endpoint>,
) -> Result<routes::interaction_message_delete::Response> {
    let body = req.inner();

    let globals = req.globals();
    let srv = req.services();
    let inter = srv.interactions.get(body.interaction_id).cast_internal()?;

    if inter.inner().token.as_ref() != Some(&body.token) {
        return Err(ApiError::with_message(
            ErrorCode::InvalidData,
            "unknown or expired interaction".to_string(),
        )
        .into());
    }

    let channel_id = inter.inner().ty.channel_id().ok_or_else(|| {
        ApiError::with_message(
            ErrorCode::InvalidData,
            "interaction type has no channel id".to_string(),
        )
    })?;

    let message_id = match body.message_id {
        InteractionMessageReq::MessageOriginal => inter.original_message_id().ok_or_else(|| {
            ApiError::with_message(
                ErrorCode::InvalidData,
                "interaction has no original message".to_string(),
            )
        })?,
        InteractionMessageReq::MessageId(id) => id,
    };

    let application_user_id: UserId = (*inter.inner().application_id).into();
    let message = srv
        .messages
        .get(channel_id, message_id, None)
        .await
        .cast_internal()?;

    if message.author_id != application_user_id {
        return Err(ApiError::from_code(ErrorCode::UnknownMessage).into());
    }

    if !message.latest_version.message_type.is_deletable() {
        return Err(ApiError::from_code(ErrorCode::CantDeleteThatMessage).into());
    }

    let chan = srv.channels.get(channel_id, None).await.cast_internal()?;
    chan.ensure_has_text()?;

    let mut data = globals.begin().await.cast_internal()?;
    data.message_delete(channel_id, message_id)
        .await
        .cast_internal()?;
    data.media_link_delete_all(message_id.into_inner())
        .await
        .cast_internal()?;
    data.commit().await.cast_internal()?;

    globals
        .messaging()
        .broadcast_channel(
            channel_id,
            MessageSync::MessageDelete {
                channel_id,
                message_id,
                room_id: chan.room_id,
            },
        )
        .await
        .cast_internal()?;

    srv.channels.invalidate(channel_id).await;

    Ok(routes::interaction_message_delete::Response {})
}

export_routes!(
    create,
    respond,
    message_create,
    message_get,
    message_edit,
    message_delete,
);
