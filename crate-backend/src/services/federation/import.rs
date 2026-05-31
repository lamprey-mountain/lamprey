use crate::error::{Error, Result};
use crate::services::federation::ServiceFederation;
use crate::types::MediaLinkType;
use common::v1::types::federation::{Hostname, Remote};
use common::v1::types::{User, UserId, UserPatch};

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
            origin_id: origin_user_id.into_inner(),
            hostname: hostname.clone(),
        };
        user.remote = Some(remote.clone());

        let mut txn = self.state.acquire_data().await?;
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
            txn = self.state.acquire_data().await?;
        }

        let mut patch = UserPatch {
            name: Some(user.name.clone()),
            description: Some(user.description.clone()),
            avatar: None,
            banner: None,
        };

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
                let media = srv
                    .media
                    .load_remote_media(
                        local_user_id,
                        Remote {
                            origin_id: origin_avatar_id.into(),
                            hostname: hostname.clone(),
                        },
                        info.cdn_url.clone(),
                    )
                    .await?;
                patch.avatar = Some(Some(media.id));
                txn.media_link_insert(media.id, *local_user_id, MediaLinkType::UserAvatar)
                    .await?;
            }
            (Some(new), Some(old)) if new == old => {
                // no op
            }
            (Some(origin_avatar_id), Some(_)) => {
                let media = srv
                    .media
                    .load_remote_media(
                        local_user_id,
                        Remote {
                            origin_id: origin_avatar_id.into(),
                            hostname: hostname.clone(),
                        },
                        info.cdn_url.clone(),
                    )
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
                let media = srv
                    .media
                    .load_remote_media(
                        local_user_id,
                        Remote {
                            origin_id: origin_banner_id.into(),
                            hostname: hostname.clone(),
                        },
                        info.cdn_url.clone(),
                    )
                    .await?;
                patch.banner = Some(Some(media.id));
                txn.media_link_insert(media.id, *local_user_id, MediaLinkType::UserBanner)
                    .await?;
            }
            (Some(new), Some(old)) if new == old => {
                // no op
            }
            (Some(origin_banner_id), Some(_)) => {
                let media = srv
                    .media
                    .load_remote_media(
                        local_user_id,
                        Remote {
                            origin_id: origin_banner_id.into(),
                            hostname: hostname.clone(),
                        },
                        info.cdn_url.clone(),
                    )
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

    // TODO: add load_remote_media -> proxy media service
    // TODO: add load_remote_invite
}
