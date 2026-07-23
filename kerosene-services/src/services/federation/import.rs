use std::sync::Arc;

use crate::error::{Error, Result};
use crate::services::federation::ServiceFederation;
use crate::services::media::Import;
use crate::types::MediaLinkType;
use common::v1::types::error::ErrorCode;
use common::v1::types::federation::signing::OutgoingRequest;
use common::v1::types::federation::{FederationEpoch, Hostname, Remote, RemoteReq};
use common::v1::types::{
    Channel, ChannelId, Invite, MediaId, Room, RoomId, User, UserId, UserPatch,
};
use common::v2::types::SERVER_USER_ID;
use common::v2::types::media::Media;
use uuid::Uuid;

impl ServiceFederation {
    /// Load a user from a remote server, fetching and caching it locally.
    pub async fn load_remote_user(
        &self,
        origin_user_id: UserId,
        hostname: &Hostname,
    ) -> Result<User> {
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
        let existing = txn.user_get_remote(&remote).await.ok();

        let local_user_id = existing.as_ref().map_or_else(UserId::new, |u| u.id);

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
            (Some(origin_avatar_id), None) => {
                let media = self
                    .load_remote_media(RemoteReq {
                        origin_id: origin_avatar_id.into(),
                        hostname: hostname.clone(),
                    })
                    .await?;
                patch.avatar = Some(Some(media.id));
                txn.media_link_insert(media.id, *local_user_id, MediaLinkType::UserAvatar)
                    .await?;
            }
            (Some(new), Some(old)) if new == old => {
                // no op
            }
            (Some(origin_avatar_id), Some(_)) => {
                let media = self
                    .load_remote_media(RemoteReq {
                        origin_id: origin_avatar_id.into(),
                        hostname: hostname.clone(),
                    })
                    .await?;
                patch.avatar = Some(Some(media.id));
                txn.media_link_delete(*local_user_id, MediaLinkType::UserAvatar)
                    .await?;
                txn.media_link_insert(media.id, *local_user_id, MediaLinkType::UserAvatar)
                    .await?;
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
            (Some(origin_banner_id), None) => {
                let media = self
                    .load_remote_media(RemoteReq {
                        origin_id: origin_banner_id.into(),
                        hostname: hostname.clone(),
                    })
                    .await?;
                patch.banner = Some(Some(media.id));
                txn.media_link_insert(media.id, *local_user_id, MediaLinkType::UserBanner)
                    .await?;
            }
            (Some(new), Some(old)) if new == old => {
                // no op
            }
            (Some(origin_banner_id), Some(_)) => {
                let media = self
                    .load_remote_media(RemoteReq {
                        origin_id: origin_banner_id.into(),
                        hostname: hostname.clone(),
                    })
                    .await?;
                patch.banner = Some(Some(media.id));
                txn.media_link_delete(*local_user_id, MediaLinkType::UserBanner)
                    .await?;
                txn.media_link_insert(media.id, *local_user_id, MediaLinkType::UserBanner)
                    .await?;
            }
        }

        // PERF: don't update if nothing changed
        txn.user_update(local_user_id, patch).await?;

        txn.commit().await?;

        Ok(user)
    }

    /// Import media from a remote server, saving a copy locally.
    pub async fn load_remote_media(&self, remote: RemoteReq<MediaId>) -> Result<Arc<Media>> {
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
                // update local data.media stuff
                todo!()
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

        // import the media data itself
        let cdn_url = info.cdn_url.join(&format!("/media/{}", remote.origin_id))?;
        let mut import = Import::new_with_id(id, SERVER_USER_ID);
        import.remote = Some(remote.with_epoch(FederationEpoch(0))); // TODO: get actual epoch
        let mut item = srv.media.import_from_url(import, &cdn_url).await?;
        Ok(item.ready().await)
    }

    /// Load an invite from a remote server, fetching and caching it locally.
    // TODO: i can't use RemoteReq<InviteCode>, i'll have to manually pass hostname/invite code?
    pub async fn load_remote_invite(&self, remote: ()) -> Result<Invite> {
        todo!()
    }

    /// Load a room from a remote server, fetching and caching it locally.
    ///
    /// rooms may require authentication to view, pass the id of a user who is able to or trying to access this room as `puppet_id`
    pub async fn load_remote_room(
        &self,
        remote: RemoteReq<RoomId>,
        puppet_id: Option<UserId>,
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
            .post(url.clone())
            .headers(req.sign(&key)?)
            .send()
            .await?;

        if !res.status().is_success() {
            return Err(Error::BadStatic("request failed"));
        }

        let room: Room = res.json().await?;

        // TODO: see crate-backend/src/services/rooms/mod.rs

        // 1. check if we have an existing Room. if the epoch matches, return it.
        // 2. fetch room from remote server
        // 3. reuse or create new id (same logic as load_user), insert or update room in database
        // 4. process media for icon, banner. import media if needed and update media links.
        // 5. return the room

        todo!()
    }

    /// Load a channel from a remote server, fetching and caching it locally.
    ///
    /// channels may require authentication to view, pass the id of a user who is able to or trying to access this channel as `puppet_id`
    pub async fn load_remote_channel(
        &self,
        remote: RemoteReq<ChannelId>,
        puppet_id: Option<UserId>,
    ) -> Result<Channel> {
        todo!()
    }
}
