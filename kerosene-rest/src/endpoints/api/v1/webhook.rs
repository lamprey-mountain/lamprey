use common::v1::types::{MessageSync, Permission, UserId, oauth::Scope};

use crate::{prelude::*, util::request::ReqAuth};

#[handler(routes::webhook_create)]
pub async fn create(
    req: Req<routes::webhook_create::Endpoint>,
) -> Result<routes::webhook_create::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();
    let mut auth = ReqAuth::new(&req);
    let webhook = srv
        .webhook
        .create(
            body.channel_id,
            &mut auth,
            body.webhook.clone(),
            body.idempotency_key.clone(),
        )
        .await
        .cast_internal()?;

    Ok(routes::webhook_create::Response { webhook })
}

#[handler(routes::webhook_list_channel)]
pub async fn list_channel(
    req: Req<routes::webhook_list_channel::Endpoint>,
) -> Result<routes::webhook_list_channel::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();
    let chan = srv
        .channels
        .get(body.channel_id, None)
        .await
        .cast_internal()?;
    chan.ensure_has_text()?;

    let mut perms = srv
        .perms
        .for_channel3(Some(user.id), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    perms.needs(Permission::IntegrationsManage);
    perms.check()?;

    let webhooks = srv
        .webhook
        .list_channel(body.channel_id, body.pagination.clone())
        .await
        .cast_internal()?;

    Ok(routes::webhook_list_channel::Response { webhooks })
}

#[handler(routes::webhook_list_room)]
pub async fn list_room(
    req: Req<routes::webhook_list_room::Endpoint>,
) -> Result<routes::webhook_list_room::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    let mut perms = srv
        .perms
        .for_room3(Some(user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    perms.needs(Permission::IntegrationsManage);
    perms.check()?;

    let webhooks = srv
        .webhook
        .list_room(body.room_id, body.pagination.clone())
        .await
        .cast_internal()?;

    Ok(routes::webhook_list_room::Response { webhooks })
}

#[handler(routes::webhook_get)]
pub async fn get(req: Req<routes::webhook_get::Endpoint>) -> Result<routes::webhook_get::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();
    let webhook = srv.webhook.get(body.webhook_id).await.cast_internal()?;

    let chan = srv
        .channels
        .get(webhook.channel_id, None)
        .await
        .cast_internal()?;
    if chan.room_id.is_none() {
        return Err(ApiError::from_code(ErrorCode::ChannelNotInRoom).into());
    }

    let mut perms = srv
        .perms
        .for_channel3(Some(user.id), webhook.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    perms.needs(Permission::IntegrationsManage);
    perms.check()?;

    chan.ensure_has_text()?;

    Ok(routes::webhook_get::Response { webhook })
}

#[handler(routes::webhook_get_with_token)]
pub async fn get_with_token(
    req: Req<routes::webhook_get_with_token::Endpoint>,
) -> Result<routes::webhook_get_with_token::Response> {
    let srv = req.services();
    let body = req.inner();
    let webhook = srv
        .webhook
        .get_with_token(body.webhook_id, &body.token)
        .await
        .cast_internal()?;

    Ok(routes::webhook_get_with_token::Response {
        webhook: webhook.into(),
    })
}

#[handler(routes::webhook_delete)]
pub async fn delete(
    req: Req<routes::webhook_delete::Endpoint>,
) -> Result<routes::webhook_delete::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();
    let mut auth = ReqAuth::new(&req);
    srv.webhook
        .delete(body.webhook_id, &mut auth)
        .await
        .cast_internal()?;

    Ok(routes::webhook_delete::Response {})
}

#[handler(routes::webhook_delete_with_token)]
pub async fn delete_with_token(
    req: Req<routes::webhook_delete_with_token::Endpoint>,
) -> Result<routes::webhook_delete_with_token::Response> {
    let srv = req.services();
    let body = req.inner();
    srv.webhook
        .delete_with_token(body.webhook_id, &body.token)
        .await
        .cast_internal()?;

    Ok(routes::webhook_delete_with_token::Response {})
}

#[handler(routes::webhook_update)]
pub async fn update(
    req: Req<routes::webhook_update::Endpoint>,
) -> Result<routes::webhook_update::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();
    let mut auth = ReqAuth::new(&req);
    let webhook = srv
        .webhook
        .update(body.webhook_id, &mut auth, body.webhook.clone())
        .await
        .cast_internal()?;

    Ok(routes::webhook_update::Response { webhook })
}

#[handler(routes::webhook_update_with_token)]
pub async fn update_with_token(
    req: Req<routes::webhook_update_with_token::Endpoint>,
) -> Result<routes::webhook_update_with_token::Response> {
    let srv = req.services();
    let body = req.inner();
    let webhook = srv
        .webhook
        .update_with_token(body.webhook_id, &body.token, body.webhook.clone())
        .await
        .cast_internal()?;

    Ok(routes::webhook_update_with_token::Response { webhook })
}

#[handler(routes::webhook_execute)]
pub async fn execute(
    req: Req<routes::webhook_execute::Endpoint>,
) -> Result<routes::webhook_execute::Response> {
    let srv = req.services();
    let body = req.inner();
    let message = srv
        .webhook
        .execute(body.webhook_id, &body.token, body.message.clone())
        .await
        .cast_internal()?;

    Ok(routes::webhook_execute::Response { message })
}

#[handler(routes::webhook_message_get)]
pub async fn message_get(
    req: Req<routes::webhook_message_get::Endpoint>,
) -> Result<routes::webhook_message_get::Response> {
    let srv = req.services();
    let body = req.inner();
    let webhook = srv
        .webhook
        .get_with_token(body.webhook_id, &body.token)
        .await
        .cast_internal()?;
    let webhook_user_id: UserId = (*webhook.id).into();

    let chan = srv
        .channels
        .get(webhook.channel_id, None)
        .await
        .cast_internal()?;
    chan.ensure_has_text()?;

    let mut data = req.globals().begin_read().await.cast_internal()?;
    let message = data
        .message_get(webhook.channel_id, body.message_id)
        .await
        .cast_internal()?;

    if message.author_id != webhook_user_id {
        return Err(ApiError::from_code(ErrorCode::UnknownMessage).into());
    }

    Ok(routes::webhook_message_get::Response { message })
}

#[handler(routes::webhook_message_edit)]
pub async fn message_edit(
    req: Req<routes::webhook_message_edit::Endpoint>,
) -> Result<routes::webhook_message_edit::Response> {
    let srv = req.services();
    let body = req.inner();
    let webhook = srv
        .webhook
        .get_with_token(body.webhook_id, &body.token)
        .await
        .cast_internal()?;
    let webhook_user_id: UserId = (*webhook.id).into();

    let chan = srv
        .channels
        .get(webhook.channel_id, None)
        .await
        .cast_internal()?;
    chan.ensure_has_text()?;

    let (_, message) = srv
        .messages
        .edit_as_webhook(
            webhook.channel_id,
            body.message_id,
            webhook_user_id,
            body.patch.clone(),
        )
        .await
        .cast_internal()?;

    Ok(routes::webhook_message_edit::Response { message })
}

#[handler(routes::webhook_message_delete)]
pub async fn message_delete(
    req: Req<routes::webhook_message_delete::Endpoint>,
) -> Result<routes::webhook_message_delete::Response> {
    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();
    let webhook = srv
        .webhook
        .get_with_token(body.webhook_id, &body.token)
        .await
        .cast_internal()?;
    let webhook_user_id: UserId = (*webhook.id).into();

    let chan = srv
        .channels
        .get(webhook.channel_id, None)
        .await
        .cast_internal()?;
    chan.ensure_has_text()?;

    let mut data = globals.begin().await.cast_internal()?;
    let message = data
        .message_get(webhook.channel_id, body.message_id)
        .await
        .cast_internal()?;

    if message.author_id != webhook_user_id {
        return Err(ApiError::from_code(ErrorCode::UnknownMessage).into());
    }
    if !message.latest_version.message_type.is_deletable() {
        return Err(ApiError::from_code(ErrorCode::CantDeleteThatMessage).into());
    }

    data.message_delete(webhook.channel_id, body.message_id)
        .await
        .cast_internal()?;
    data.media_link_delete_all(body.message_id.into_inner())
        .await
        .cast_internal()?;
    data.commit().await.cast_internal()?;

    globals
        .messaging()
        .broadcast_channel(
            webhook.channel_id,
            MessageSync::MessageDelete {
                channel_id: webhook.channel_id,
                message_id: body.message_id,
                room_id: chan.room_id,
            },
        )
        .await
        .cast_internal()?;

    srv.channels.invalidate(webhook.channel_id).await;

    Ok(routes::webhook_message_delete::Response {})
}

export_routes!(
    create,
    list_channel,
    list_room,
    get,
    get_with_token,
    delete,
    delete_with_token,
    update,
    update_with_token,
    execute,
    message_get,
    message_edit,
    message_delete,
);
