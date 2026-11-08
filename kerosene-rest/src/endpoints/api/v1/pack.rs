use common::v1::types::{Permission, RoomType, SERVER_ROOM_ID, oauth::Scope};
use lamprey_backend_services::compat::types::DbRoomCreate;

use crate::{prelude::*, util::request::ReqAuth};

#[handler(routes::pack_create)]
pub async fn create(
    req: Req<routes::pack_create::Endpoint>,
) -> Result<routes::pack_create::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();

    srv.perms
        .for_room3(Some(user.id), SERVER_ROOM_ID)
        .await
        .cast_internal()?
        .ensure_view()?
        .needs(Permission::RoomCreate)
        .check()?;

    if let Some(media_id) = body.pack.icon {
        let media = globals
            .begin_read()
            .await
            .cast_internal()?
            .media_select(media_id)
            .await
            .cast_internal()?;
        if !media.metadata.is_image() {
            return Err(ApiError::from_code(ErrorCode::MediaNotAnImage).into());
        }
    }

    let extra = DbRoomCreate {
        id: None,
        ty: RoomType::Emoji,
        welcome_channel_id: None,
        remote: None,
    };

    let mut auth = ReqAuth::new(&req);
    let handle = srv
        .rooms
        .create(
            body.pack.clone(),
            &mut auth,
            extra,
            body.idempotency_key.clone(),
        )
        .await
        .cast_internal()?;
    let loaded = handle.ready(true).await.cast_internal()?;
    let pack = (*loaded.room).clone();

    Ok(routes::pack_create::Response { pack })
}

#[handler(routes::pack_upgrade)]
pub async fn upgrade(
    _req: Req<routes::pack_upgrade::Endpoint>,
) -> Result<routes::pack_upgrade::Response> {
    Err(Error::Unimplemented)
}

#[handler(routes::pack_import)]
pub async fn import(
    _req: Req<routes::pack_import::Endpoint>,
) -> Result<routes::pack_import::Response> {
    Err(Error::Unimplemented)
}

#[handler(routes::pack_export)]
pub async fn export(
    _req: Req<routes::pack_export::Endpoint>,
) -> Result<routes::pack_export::Response> {
    Err(Error::Unimplemented)
}

#[handler(routes::pack_user_list)]
pub async fn user_list(
    _req: Req<routes::pack_user_list::Endpoint>,
) -> Result<routes::pack_user_list::Response> {
    Err(Error::Unimplemented)
}

#[handler(routes::pack_user_install)]
pub async fn user_install(
    _req: Req<routes::pack_user_install::Endpoint>,
) -> Result<routes::pack_user_install::Response> {
    Err(Error::Unimplemented)
}

#[handler(routes::pack_user_uninstall)]
pub async fn user_uninstall(
    _req: Req<routes::pack_user_uninstall::Endpoint>,
) -> Result<routes::pack_user_uninstall::Response> {
    Err(Error::Unimplemented)
}

#[handler(routes::pack_room_list)]
pub async fn room_list(
    _req: Req<routes::pack_room_list::Endpoint>,
) -> Result<routes::pack_room_list::Response> {
    Err(Error::Unimplemented)
}

#[handler(routes::pack_room_install)]
pub async fn room_install(
    _req: Req<routes::pack_room_install::Endpoint>,
) -> Result<routes::pack_room_install::Response> {
    Err(Error::Unimplemented)
}

#[handler(routes::pack_room_uninstall)]
pub async fn room_uninstall(
    _req: Req<routes::pack_room_uninstall::Endpoint>,
) -> Result<routes::pack_room_uninstall::Response> {
    Err(Error::Unimplemented)
}

export_routes!(
    create,
    upgrade,
    import,
    export,
    user_list,
    user_install,
    user_uninstall,
    room_list,
    room_install,
    room_uninstall,
);
