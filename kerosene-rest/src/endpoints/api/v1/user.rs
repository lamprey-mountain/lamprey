use common::{
    v1::types::{
        MessageSync, UserWithRelationship,
        ack::{AckBulkItem, AckType},
        oauth::Scope,
        util::Time,
    },
    v2::types::MessageId,
};
use http::StatusCode;
use lamprey_backend_services::{compat::types::UserIdReq, services::messages::create2::Create};
use tracing::warn;

use crate::prelude::*;

#[handler(routes::user_get)]
pub async fn get(req: Req<routes::user_get::Endpoint>) -> Result<routes::user_get::Response> {
    let srv = req.services();
    let identity = req.identity();

    let mut user = match &req.inner().user_id {
        UserIdReq::UserSelf => identity.ensure_user()?.clone(),
        UserIdReq::UserId(target_user_id) => srv
            .users
            .get(*target_user_id, identity.user_id())
            .await
            .cast_internal()?,
        UserIdReq::RemoteUser(user_id, hostname) => {
            // TODO: get local user, get server info, check if user remote epoch == server sync epoch
            // NOTE: do i put epoch checks in users service, federation service, somewhere else...?
            srv.federation
                .import_user(*user_id, &hostname)
                .await
                .cast_internal()?
        }
    };

    if identity.ensure_scopes(&[Scope::Email]).is_err() {
        user.emails = None;
    }

    // TODO: move relationship fetching to users service?
    let relationship = if let Some(auth_user) = identity.user() {
        req.globals()
            .begin_read()
            .await
            .cast_internal()?
            .user_relationship_get(auth_user.id, user.id)
            .await
            .cast_internal()?
            .unwrap_or_default()
    } else {
        Default::default()
    };

    let user = UserWithRelationship {
        inner: user,
        relationship,
    };

    Ok(routes::user_get::Response { user })
}

export_routes!(get);
