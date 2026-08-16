use axum::{Json, extract::State, response::IntoResponse};
use common::v1::routes;
use common::v1::types::oauth::Scope;
use common::v1::types::{Permission, RoomType};
use common::v2::types::SERVER_ROOM_ID;
use http::StatusCode;
use kerosene_core::error::{ApiError, ErrorCode};
use kerosene_services::globals::server_state::ServerState;
use lamprey_backend_data_postgres::{DbRoomCreate, MediaLinkType};
use lamprey_macros::handler;
use tracing::debug;

use crate::routes::util::auth::Auth4;
use crate::routes2;
use utoipa_axum::router::OpenApiRouter;

use crate::prelude::*;

#[handler(routes::pack_create)]
async fn pack_create(
    mut auth: Auth4,
    State(globals): State<Globals>,
    req: routes::pack_create::Request,
) -> Result<impl IntoResponse> {
    let user = auth.ensure_user()?;
    user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;

    let srv = globals.services();
    let perms = srv
        .perms
        .for_room3(Some(user.id), SERVER_ROOM_ID)
        .await?
        .ensure_view()?
        .needs(Permission::RoomCreate) // NOTE: should i have a different permission for emoji packs?
        .check()?;

    debug!("server perms for {}: {:?}", user.id, perms);

    let icon = req.pack.icon;
    if let Some(media_id) = icon {
        let mut data = globals.begin_read().await?;
        let media = data.media_select(media_id).await?;
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
    let room = srv
        .rooms
        .create(req.pack, &mut auth, extra, req.idempotency_key)
        .await?;
    if let Some(media_id) = icon {
        let mut txn = globals.begin().await?;
        txn.media_link_create_exclusive(media_id, *room.id, MediaLinkType::RoomIcon)
            .await?;
        txn.commit().await?;
    }

    Ok((StatusCode::CREATED, Json(room)))
}

// TODO: impl packs

#[handler(routes::pack_upgrade)]
async fn pack_upgrade(
    _auth: Auth4,
    State(_globals): State<Globals>,
    _req: routes::pack_upgrade::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

#[handler(routes::pack_import)]
async fn pack_import(
    _auth: Auth4,
    State(_globals): State<Globals>,
    _req: routes::pack_import::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

#[handler(routes::pack_export)]
async fn pack_export(
    _auth: Auth4,
    State(_globals): State<Globals>,
    _req: routes::pack_export::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

#[handler(routes::pack_user_list)]
async fn pack_user_list(
    _auth: Auth4,
    State(_globals): State<Globals>,
    _req: routes::pack_user_list::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

#[handler(routes::pack_user_install)]
async fn pack_user_install(
    _auth: Auth4,
    State(_globals): State<Globals>,
    _req: routes::pack_user_install::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

#[handler(routes::pack_user_uninstall)]
async fn pack_user_uninstall(
    _auth: Auth4,
    State(_globals): State<Globals>,
    _req: routes::pack_user_uninstall::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

#[handler(routes::pack_room_list)]
async fn pack_room_list(
    _auth: Auth4,
    State(_globals): State<Globals>,
    _req: routes::pack_room_list::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

#[handler(routes::pack_room_install)]
async fn pack_room_install(
    _auth: Auth4,
    State(_globals): State<Globals>,
    _req: routes::pack_room_install::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

#[handler(routes::pack_room_uninstall)]
async fn pack_room_uninstall(
    _auth: Auth4,
    State(_globals): State<Globals>,
    _req: routes::pack_room_uninstall::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(pack_create))
        .routes(routes2!(pack_upgrade))
        .routes(routes2!(pack_import))
        .routes(routes2!(pack_export))
        .routes(routes2!(pack_user_list))
        .routes(routes2!(pack_user_install))
        .routes(routes2!(pack_user_uninstall))
        .routes(routes2!(pack_room_list))
        .routes(routes2!(pack_room_install))
        .routes(routes2!(pack_room_uninstall))
}
