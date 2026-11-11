use common::v1::types::{Permission, oauth::Scope, tag::TagQueryResponse};

use crate::{prelude::*, util::request::ReqAuth};

#[handler(routes::tag_create)]
pub async fn create(
    req: Req<routes::tag_create::Endpoint>,
) -> Result<routes::tag_create::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();
    body.tag
        .validate()
        .map_err(|e| ApiError::with_message(ErrorCode::InvalidData, e.to_string()))?;

    srv.perms
        .for_channel3(Some(user.id), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .needs(Permission::ChannelEdit)
        .check()?;

    let mut auth = ReqAuth::new(&req);
    let tag = srv
        .tag
        .create(
            body.channel_id,
            &mut auth,
            body.tag.clone(),
            body.idempotency_key.clone(),
        )
        .await
        .cast_internal()?;

    Ok(routes::tag_create::Response { tag })
}

#[handler(routes::tag_update)]
pub async fn update(
    req: Req<routes::tag_update::Endpoint>,
) -> Result<routes::tag_update::Response> {
    let identity = req.identity();
    identity.ensure_user()?.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();
    body.patch
        .validate()
        .map_err(|e| ApiError::with_message(ErrorCode::InvalidData, e.to_string()))?;

    srv.perms
        .for_channel3(identity.user_id(), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .needs(Permission::ChannelEdit)
        .check()?;

    let mut auth = ReqAuth::new(&req);
    let tag = srv
        .tag
        .update(body.channel_id, body.tag_id, &mut auth, body.patch.clone())
        .await
        .cast_internal()?;

    Ok(routes::tag_update::Response { tag })
}

#[handler(routes::tag_delete)]
pub async fn delete(
    req: Req<routes::tag_delete::Endpoint>,
) -> Result<routes::tag_delete::Response> {
    let identity = req.identity();
    identity.ensure_user()?.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    srv.perms
        .for_channel3(identity.user_id(), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .needs(Permission::ChannelEdit)
        .check()?;

    let mut auth = ReqAuth::new(&req);
    srv.tag
        .delete(body.channel_id, body.tag_id, &mut auth, body.query.force)
        .await
        .cast_internal()?;

    Ok(routes::tag_delete::Response {})
}

#[handler(routes::tag_list)]
pub async fn list(req: Req<routes::tag_list::Endpoint>) -> Result<routes::tag_list::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    srv.perms
        .for_channel3(identity.user_id(), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .check()?;

    let tags = req
        .globals()
        .begin_read()
        .await
        .cast_internal()?
        .tag_list(body.channel_id, body.list.archived, body.pagination.clone())
        .await
        .cast_internal()?;

    Ok(routes::tag_list::Response { tags })
}

#[handler(routes::tag_get)]
pub async fn get(req: Req<routes::tag_get::Endpoint>) -> Result<routes::tag_get::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    srv.perms
        .for_channel3(identity.user_id(), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .check()?;

    let tag = srv.tag.get(body.tag_id).await.cast_internal()?;

    Ok(routes::tag_get::Response { tag })
}

#[handler(routes::tag_search)]
pub async fn search(
    req: Req<routes::tag_search::Endpoint>,
) -> Result<routes::tag_search::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    srv.perms
        .for_channel3(identity.user_id(), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .check()?;

    let tags = req
        .globals()
        .begin_read()
        .await
        .cast_internal()?
        .tag_search(
            body.channel_id,
            body.search.query.clone(),
            body.search.archived,
            body.pagination.clone(),
        )
        .await
        .cast_internal()?;

    Ok(routes::tag_search::Response { tags })
}

#[handler(routes::tag_query)]
pub async fn query(req: Req<routes::tag_query::Endpoint>) -> Result<routes::tag_query::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();
    body.body
        .validate()
        .map_err(|e| ApiError::with_message(ErrorCode::InvalidData, e.to_string()))?;

    srv.perms
        .for_channel3(identity.user_id(), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .check()?;

    let tags = req
        .globals()
        .begin_read()
        .await
        .cast_internal()?
        .tag_get_many(body.channel_id, &body.body.tag_ids)
        .await
        .cast_internal()?;

    Ok(routes::tag_query::Response {
        body: TagQueryResponse { tags },
    })
}

export_routes!(create, update, delete, list, get, search, query);
