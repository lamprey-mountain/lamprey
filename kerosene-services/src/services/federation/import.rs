use std::sync::Arc;

use crate::error::{Error, Result};
use crate::services::federation::ServiceFederation;
use crate::services::media::Import;
use crate::types::MediaLinkType;
use common::v1::types::error::ErrorCode;
use common::v1::types::federation::signing::OutgoingRequest;
use common::v1::types::federation::{FederationEpoch, Hostname, Remote, RemoteReq};
use common::v1::types::{
    Channel, ChannelId, ChannelPatch, Invite, InviteCode, MediaId, Room, RoomId, RoomPatch, User,
    UserId, UserPatch,
};
use common::v2::types::SERVER_USER_ID;
use common::v2::types::media::Media;
use uuid::Uuid;

// TODO: use epochs for caching

// FIXME: handle recursive types
// rooms have welcome_channel_id
// channels have room_id
// i need to make sure that channels/rooms dont infinitely import each other

impl ServiceFederation {
    /// Load a user from a remote server, fetching and caching it locally.
    pub async fn import_user(&self, origin_user_id: UserId, hostname: &Hostname) -> Result<User> {
        let info = self.fetch_server_info(hostname).await?;
        let url = info
            .api_url
            .join(&format!("/api/v1/user/{}", origin_user_id))?;

        let res = self.state.services().http.client.get(url).send().await?;
        if !res.status().is_success() {
            return Err(Error::BadStatic("failed to fetch remote user"));
        }

        let mut user: User = res.json().await?;
        let remote = Remote {
            origin_id: origin_user_id,
            hostname: hostname.clone(),
            epoch: FederationEpoch(0),
        };
        user.remote = Some(remote.clone());

        let mut txn = self.state.begin().await?;
        let srv = self.state.services();
        let local = txn.user_get(origin_user_id).await.ok();
        let existing = txn.user_get_remote(&remote).await.ok();

        let local_user_id = match (&local, &existing) {
            // we already have a local copy, update it
            (_, Some(existing)) => existing.id,

            // try to use the same id as origin
            (None, None) => origin_user_id,

            // it's already taken, create a new id
            (Some(_), None) => UserId::new(),
        };

        if existing.is_none() {
            txn.user_create(crate::types::DbUserCreate {
                id: Some(local_user_id),
                parent_id: None,
                name: user.name.clone(),
                description: user.description.clone(),
                puppet: user.puppet.clone(),
                registered_at: user.registered_at,
                system: user.system,
                remote: Some(remote.clone()),
            })
            .await?;

            // commit so that the media service sees the user
            txn.commit().await?;
            txn = self.state.begin().await?;
        }

        let mut patch = UserPatch {
            name: Some(user.name.clone()),
            description: Some(user.description.clone()),
            avatar: None,
            banner: None,
        };

        // PERF: run multiple media imports in parallel
        match (user.avatar, existing.as_ref().and_then(|e| e.avatar)) {
            (None, None) => {
                // no op
            }
            (None, Some(_)) => {
                patch.avatar = Some(None);
                txn.media_link_delete(*local_user_id, MediaLinkType::UserAvatar)
                    .await?;
            }
            (Some(origin_avatar_id), existing_avatar) => {
                let media = self
                    .import_media(RemoteReq {
                        origin_id: origin_avatar_id.into(),
                        hostname: hostname.clone(),
                    })
                    .await?;

                if Some(media.id) != existing_avatar {
                    patch.avatar = Some(Some(media.id));
                    if existing.is_some() {
                        txn.media_link_delete(*local_user_id, MediaLinkType::UserAvatar)
                            .await?;
                    }
                    txn.media_link_insert(media.id, *local_user_id, MediaLinkType::UserAvatar)
                        .await?;
                }
            }
        }

        // theres probably some way to deduplicate this code
        match (user.banner, existing.as_ref().and_then(|e| e.banner)) {
            (None, None) => {
                // no op
            }
            (None, Some(_)) => {
                patch.banner = Some(None);
                txn.media_link_delete(*local_user_id, MediaLinkType::UserBanner)
                    .await?;
            }
            (Some(origin_banner_id), existing_banner) => {
                let media = self
                    .import_media(RemoteReq {
                        origin_id: origin_banner_id.into(),
                        hostname: hostname.clone(),
                    })
                    .await?;

                if Some(media.id) != existing_banner {
                    patch.banner = Some(Some(media.id));
                    if existing_banner.is_some() {
                        txn.media_link_delete(*local_user_id, MediaLinkType::UserBanner)
                            .await?;
                    }
                    txn.media_link_insert(media.id, *local_user_id, MediaLinkType::UserBanner)
                        .await?;
                }
            }
        }

        // PERF: don't update if nothing changed
        txn.user_update(local_user_id, patch).await?;

        txn.commit().await?;

        Ok(user)
    }

    /// Import media from a remote server, saving a copy locally.
    pub async fn import_media(&self, remote: RemoteReq<MediaId>) -> Result<Arc<Media>> {
        let srv = self.state.services();

        // fetch remote media object
        let info = self.fetch_server_info(&remote.hostname).await?;
        let url = info
            .api_url
            .join(&format!("/api/v1/media/{}", remote.origin_id))?;
        let res = srv.http.client.get(url).send().await?;
        if !res.status().is_success() {
            return Err(Error::BadStatic("failed to fetch remote media"));
        }
        let media: Media = res.json().await?;

        // check existing media
        let existing = match srv.media.get_remote(&remote).await {
            Ok(media) => Some(media),
            Err(Error::ApiError(err)) if err.code == ErrorCode::UnknownMedia => None,
            Err(err) => return Err(err),
        };
        if let Some(existing) = existing {
            if existing.media().version_id == media.version_id {
                // NOTE: i would need to bump epoch later?
                return Ok(existing.media());
            } else {
                let new_media = Arc::new(Media {
                    version_id: media.version_id,
                    filename: media.filename,
                    alt: media.alt,
                    // TODO: handle strip_exif somehow?
                    // TODO: maybe handle updating other fields?
                    ..(*existing.media()).clone()
                });

                let mut txn = self.state.begin().await?;
                txn.media_replace((*new_media).clone()).await?;
                txn.commit().await?;
                return Ok(new_media);
            }
        }

        // we don't have the remote media cached locally, begin importing
        let id = match &existing {
            Some(m) => m.media().id,
            None => {
                // check for id collision
                let id_available = srv.media.get(media.id).await.is_err_and(|err| match err {
                    Error::ApiError(err) => err.code == ErrorCode::UnknownMedia,
                    _ => false,
                });
                if id_available {
                    media.id
                } else {
                    MediaId::new()
                }
            }
        };

        // TODO: import media.user_id

        // import the media data itself
        let cdn_url = info.cdn_url.join(&format!("/media/{}", remote.origin_id))?;
        let mut import = Import::new_with_id(id, SERVER_USER_ID);
        import.remote = Some(remote.with_epoch(FederationEpoch(0))); // TODO: get actual epoch
        let mut item = srv.media.import_from_url(import, &cdn_url).await?;
        Ok(item.ready().await)
    }

    /// Load an invite from a remote server, fetching and caching it locally.
    pub async fn import_invite(&self, hostname: &Hostname, code: &InviteCode) -> Result<Invite> {
        let info = self.fetch_server_info(&hostname).await?;
        let url = info.api_url.join(&format!("/api/v1/invite/{}", code))?;

        let key = self
            .get_local_keys()
            .await
            .into_iter()
            .next()
            .ok_or_else(|| Error::BadStatic("no local signing keys"))?;

        let req = OutgoingRequest {
            origin: &self.state.config().hostname2()?,
            host: &hostname,
            method: "GET",
            path: url.path(),
            body: &[],
        };

        let res = self
            .state
            .services()
            .http
            .client
            .get(url.clone())
            .headers(req.sign(&key)?)
            .send()
            .await?;

        if !res.status().is_success() {
            return Err(Error::BadStatic("request failed"));
        }

        let invite: Invite = res.json().await?;

        // TODO: import invite similarly to load_remote_room

        todo!()
    }

    /// Load a room from a remote server, fetching and caching it locally.
    ///
    /// rooms may require authentication to view, pass the id of a user who is able to or trying to access this room as `puppet_id`
    // TODO: use puppet_id
    pub async fn import_room(
        &self,
        remote: RemoteReq<RoomId>,
        _puppet_id: Option<UserId>,
    ) -> Result<Room> {
        let info = self.fetch_server_info(&remote.hostname).await?;
        let url = info
            .api_url
            .join(&format!("/api/v1/room/{}", remote.origin_id))?;

        let key = self
            .get_local_keys()
            .await
            .into_iter()
            .next()
            .ok_or_else(|| Error::BadStatic("no local signing keys"))?;

        let req = OutgoingRequest {
            origin: &self.state.config().hostname2()?,
            host: &remote.hostname,
            method: "GET",
            path: url.path(),
            body: &[],
        };

        let res = self
            .state
            .services()
            .http
            .client
            .get(url.clone())
            .headers(req.sign(&key)?)
            .send()
            .await?;

        if !res.status().is_success() {
            return Err(Error::BadStatic("failed to fetch remote room"));
        }

        let mut room: Room = res.json().await?;
        let remote_info = Remote {
            origin_id: remote.origin_id,
            hostname: remote.hostname.clone(),
            epoch: FederationEpoch(0),
        };
        room.remote = Some(remote_info.clone());

        let mut txn = self.state.begin().await?;
        let local = txn.room_get(remote.origin_id).await.ok();
        let existing = txn.room_get_remote(&remote).await.ok();

        let local_room_id = match (&local, &existing) {
            // we already have a local copy, update it
            (_, Some(existing)) => existing.id,

            // try to use the same id as origin
            (None, None) => remote.origin_id,

            // it's already taken, create a new id
            (Some(_), None) => RoomId::new(),
        };

        // TODO: use room service to create room

        if existing.is_none() {
            txn.room_create(
                crate::types::RoomCreate {
                    name: room.name.clone(),
                    description: room.description.clone(),
                    icon: None,
                    banner: None,
                    public: Some(room.public),
                },
                crate::types::DbRoomCreate {
                    id: Some(local_room_id),
                    ty: room.room_type,
                    welcome_channel_id: room.welcome_channel_id,
                    remote: Some(remote_info.clone()),
                },
            )
            .await?;

            // commit so media service sees the room
            txn.commit().await?;
            txn = self.state.begin().await?;
        }

        let mut patch = RoomPatch {
            name: Some(room.name.clone()),
            description: None,
            icon: None,
            banner: None,
            public: None,
            // TODO: other fields
            // welcome_channel_id: Some(Some(room.welcome_channel_id)),
            // afk_channel_id: Some(Some(room.afk_channel_id)),
            // afk_channel_timeout: Some(Some(room.afk_channel_timeout)),
            // invites_paused_until: Some(Some(room.invites_paused_until)),
            welcome_channel_id: None,
            afk_channel_id: None,
            afk_channel_timeout: None,
            invites_paused_until: None,
        };

        // PERF: run multiple media imports in parallel
        // NOTE: we compare against remote origin ids for icon/banner,
        // but we don't store them in DB. Re-fetching is currently unavoidable.
        match (room.icon, existing.as_ref().and_then(|e| e.icon)) {
            (None, None) => {}
            (None, Some(_)) => {
                patch.icon = Some(None);
                txn.media_link_delete(*local_room_id, MediaLinkType::RoomIcon)
                    .await?;
            }
            (Some(origin_icon_id), existing_icon) => {
                let media = self
                    .import_media(RemoteReq {
                        origin_id: origin_icon_id.into(),
                        hostname: remote.hostname.clone(),
                    })
                    .await?;

                if Some(media.id) != existing_icon {
                    patch.icon = Some(Some(media.id));
                    if existing_icon.is_some() {
                        txn.media_link_delete(*local_room_id, MediaLinkType::RoomIcon)
                            .await?;
                    }
                    txn.media_link_insert(media.id, *local_room_id, MediaLinkType::RoomIcon)
                        .await?;
                }
            }
        }

        match (room.banner, existing.as_ref().and_then(|e| e.banner)) {
            (None, None) => {}
            (None, Some(_)) => {
                patch.banner = Some(None);
                txn.media_link_delete(*local_room_id, MediaLinkType::RoomBanner)
                    .await?;
            }
            (Some(origin_banner_id), existing_banner) => {
                let media = self
                    .import_media(RemoteReq {
                        origin_id: origin_banner_id.into(),
                        hostname: remote.hostname.clone(),
                    })
                    .await?;
                if Some(media.id) != existing_banner {
                    patch.banner = Some(Some(media.id));
                    if existing_banner.is_some() {
                        txn.media_link_delete(*local_room_id, MediaLinkType::RoomBanner)
                            .await?;
                    }
                    txn.media_link_insert(media.id, *local_room_id, MediaLinkType::RoomBanner)
                        .await?;
                }
            }
        }

        // TODO: mirror room.welcome_channel_id, room.afk_channel_id, room.owner_id

        txn.room_update(local_room_id, patch).await?;
        txn.commit().await?;

        room.id = local_room_id;

        Ok(room)
    }

    /// Load a channel from a remote server, fetching and caching it locally.
    ///
    /// channels may require authentication to view, pass the id of a user who is able to or trying to access this channel as `puppet_id`
    pub async fn import_channel(
        &self,
        remote: RemoteReq<ChannelId>,
        _puppet_id: Option<UserId>,
    ) -> Result<Channel> {
        let info = self.fetch_server_info(&remote.hostname).await?;
        let url = info
            .api_url
            .join(&format!("/api/v1/channel/{}", remote.origin_id))?;

        let key = self
            .get_local_keys()
            .await
            .into_iter()
            .next()
            .ok_or_else(|| Error::BadStatic("no local signing keys"))?;

        let req = OutgoingRequest {
            origin: &self.state.config().hostname2()?,
            host: &remote.hostname,
            method: "GET",
            path: url.path(),
            body: &[],
        };

        let res = self
            .state
            .services()
            .http
            .client
            .get(url.clone())
            .headers(req.sign(&key)?)
            .send()
            .await?;

        if !res.status().is_success() {
            return Err(Error::BadStatic("failed to fetch remote channel"));
        }

        let mut channel: Channel = res.json().await?;
        let remote_info = Remote {
            origin_id: remote.origin_id,
            hostname: remote.hostname.clone(),
            epoch: FederationEpoch(0),
        };
        channel.remote = Some(remote_info.clone());

        let mut txn = self.state.begin().await?;
        let local = txn.channel_get(remote.origin_id).await.ok();
        let existing = txn.channel_get_remote(&remote).await.ok();

        let local_channel_id = match (&local, &existing) {
            // we already have a local copy, update it
            (_, Some(existing)) => existing.id,

            // try to use the same id as origin
            (None, None) => remote.origin_id,

            // it's already taken, create a new id
            (Some(_), None) => ChannelId::new(),
        };

        // TODO: use channel service to create channel

        if existing.is_none() {
            txn.channel_create_with_id(
                local_channel_id,
                lamprey_backend_data_postgres::DbChannelCreate {
                    // room_id: channel
                    //     .room_id
                    //     .ok_or(Error::BadStatic("channel must have room_id"))?,
                    name: channel.name.clone(),
                    description: channel.description.clone(),
                    url: channel.url.clone(),
                    ty: channel.ty.into(),
                    nsfw: channel.nsfw,
                    bitrate: channel.bitrate.map(|v| v as i32),
                    user_limit: channel.user_limit.map(|v| v as i32),
                    invitable: channel.invitable,
                    auto_archive_duration: channel.auto_archive_duration.map(|v| v as i64),
                    default_auto_archive_duration: channel
                        .default_auto_archive_duration
                        .map(|v| v as i64),
                    slowmode_thread: channel.slowmode_thread.map(|v| v as i64),
                    slowmode_message: channel.slowmode_message.map(|v| v as i64),
                    default_slowmode_message: channel.default_slowmode_message.map(|v| v as i64),

                    icon: None,                     // handle icon later
                    parent_id: None,                // FIXME: import parent_id
                    locked: false, // FIXME: import locked (role ids need to be imported)
                    tags: None,    // FIXME: import tags
                    creator_id: channel.creator_id, // FIXME: import creator_id
                    owner_id: None, // FIXME: import owner_id
                    room_id: None, // FIXME: import room_id
                },
            )
            .await?;

            // commit so media service sees the channel
            txn.commit().await?;
            txn = self.state.begin().await?;
        }

        let mut patch = ChannelPatch::default();

        match (channel.icon, existing.as_ref().and_then(|e| e.icon)) {
            (None, None) => {}
            (None, Some(_)) => {
                patch.icon = Some(None);
                txn.media_link_delete(*local_channel_id, MediaLinkType::ChannelIcon)
                    .await?;
            }
            (Some(origin_icon_id), existing_icon) => {
                let media = self
                    .import_media(RemoteReq {
                        origin_id: origin_icon_id.into(),
                        hostname: remote.hostname.clone(),
                    })
                    .await?;

                if Some(media.id) != existing_icon {
                    patch.icon = Some(Some(media.id));
                    if existing_icon.is_some() {
                        txn.media_link_delete(*local_channel_id, MediaLinkType::ChannelIcon)
                            .await?;
                    }
                    txn.media_link_insert(media.id, *local_channel_id, MediaLinkType::ChannelIcon)
                        .await?;
                }
            }
        }

        txn.channel_update(local_channel_id, patch).await?;
        txn.commit().await?;

        channel.id = local_channel_id;

        Ok(channel)
    }
}

// TODO: create a trait or utility or something to reduce boilerplate
// pub trait Syncable: Diff {
//     // trait Syncable: Diff
//     type Patch;
//
//     fn sync(&self) {
//         // local, existing
//     }
// }
