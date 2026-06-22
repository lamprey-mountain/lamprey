use crate::bridge::{BridgeHandle, PortalHandle};
use crate::lamprey::ChannelId;
use crate::prelude::*;
use common::util::Diff;
use common::v1::types::misc::{ApplicationIdReq, UserIdReq};
use common::v1::types::{
    Message, PaginationDirection, PaginationQuery, PuppetCreate, User, UserPatch,
};
use common::v2::types::media::{Media, MediaCreate, MediaCreateSource, MediaDoneParams};
use common::v2::types::{MessageId, UserId};
use url::Url;

/// wrapper around the lamprey client for common operations
pub struct LampreyClient {
    pub http: sdk::http::Http,
    pub bridge: BridgeHandle,
    pub channel_id: ChannelId,
}

impl LampreyClient {
    pub fn new(
        http: sdk::http::Http,
        bridge: BridgeHandle,
        channel_id: ChannelId,
    ) -> Self {
        Self {
            http,
            bridge,
            channel_id,
        }
    }

    // sync puppet, return puppet user
    pub async fn sync_puppet_discord(&self, message: &discord::Message) -> Result<User> {
        let mut puppet = self
            .http
            .puppet_ensure(
                ApplicationIdReq::AppSelf,
                message.author.id.to_string(),
                &PuppetCreate {
                    name: message.author.display_name().to_owned(),
                    description: None,
                    bot: message.author.bot,
                    system: message.author.system,
                },
            )
            .await?;

        let mut patch = UserPatch::default();

        let puppet_db = self
            .bridge
            .db
            .puppet_get_by_lamprey_id(puppet.id.to_string())
            .await?;
        let puppet_db = if let Some(mut puppet_db) = puppet_db {
            // check avatar
            match (
                puppet_db.discord_avatar_url.as_ref(),
                message.author.avatar_url(),
            ) {
                (None, None) => {
                    // user doesn't have a pfp
                }
                (Some(old), Some(new)) if *old == new => {
                    // pfp didn't change
                }
                (Some(_), None) => {
                    // user removed their pfp
                    patch.avatar = Some(None);
                }
                (_, Some(new)) => {
                    // (None, Some): user set pfp; upload media
                    // (Some, some): user changed pfp; upload media
                    let media = self
                        .import_url(ImportUrl {
                            url: new.parse().unwrap(),
                            filename: None,
                            description: None,
                            size: None,
                            user_id: Some(puppet.id),
                        })
                        .await?;
                    patch.avatar = Some(Some(media.id));
                }
            }

            // check banner
            match (
                puppet_db.discord_banner_url.as_ref(),
                message.author.banner_url(),
            ) {
                (None, None) => {
                    // user doesn't have a banner
                }
                (Some(old), Some(new)) if *old == new => {
                    // banner didn't change
                }
                (Some(_), None) => {
                    // user removed their banner
                    patch.banner = Some(None);
                }
                (_, Some(new)) => {
                    // (None, Some): user set banner; upload media
                    // (Some, some): user changed banner; upload media
                    let media = self
                        .import_url(ImportUrl {
                            url: new.parse().unwrap(),
                            filename: None,
                            description: None,
                            size: None,
                            user_id: Some(puppet.id),
                        })
                        .await?;
                    patch.banner = Some(Some(media.id));
                }
            }

            puppet_db.discord_avatar_url = message.author.avatar_url();
            puppet_db.discord_banner_url = message.author.banner_url();
            puppet_db
        } else {
            // upload avatar, banner
            let avatar = if let Some(url) = message.author.avatar_url() {
                let media = self
                    .import_url(ImportUrl {
                        url: url.parse().unwrap(),
                        filename: None,
                        description: None,
                        size: None,
                        user_id: Some(puppet.id),
                    })
                    .await?;
                Some(Some(media.id))
            } else {
                None
            };

            let banner = if let Some(url) = message.author.banner_url() {
                let media = self
                    .import_url(ImportUrl {
                        url: url.parse().unwrap(),
                        filename: None,
                        description: None,
                        size: None,
                        user_id: Some(puppet.id),
                    })
                    .await?;
                Some(Some(media.id))
            } else {
                None
            };

            patch.avatar = avatar;
            patch.banner = banner;

            bridge::User {
                source_platform: bridge::Platform::Discord,
                lamprey_id: puppet.id,
                discord_id: message.author.id,
                discord_avatar_url: message.author.avatar_url(),
                discord_banner_url: message.author.banner_url(),
            }
        };

        if patch.changes(&puppet) {
            self.http
                .user_update(UserIdReq::UserId(puppet.id), &patch)
                .await?;
            self.bridge.db.puppet_create(puppet_db).await?;
            puppet = patch.apply(puppet);
        }

        // TODO: sync guild member nickname -> room member override name
        // probably need to add a new table to support this?

        Ok(puppet)
    }

    /// import media from a url
    pub async fn import_url(&self, import: ImportUrl) -> Result<Media> {
        let http = if let Some(user_id) = import.user_id {
            self.http.for_puppet(user_id)?
        } else {
            self.http.clone()
        };

        let created = http
            .media_create(&MediaCreate {
                strip_exif: false,
                alt: import.description,
                source: MediaCreateSource::Download {
                    filename: import.filename,
                    size: import.size,
                    source_url: import.url,
                },
            })
            .await?;
        let media = http
            .media_done(
                created.media_id,
                &MediaDoneParams {
                    process_async: false,
                },
            )
            .await?
            .expect("media should be processed synchronously");
        Ok(media)
    }

    pub async fn fetch_after(&self, id: MessageId) -> Result<Vec<Message>> {
        let page = self
            .http
            .message_list(
                self.channel_id,
                &PaginationQuery {
                    from: Some(id),
                    to: None,
                    dir: Some(PaginationDirection::F),
                    limit: Some(256),
                },
            )
            .await?;
        Ok(page.items)
    }
}

pub struct ImportUrl {
    pub url: Url,
    pub filename: Option<String>,
    pub description: Option<String>,
    pub size: Option<u64>,
    pub user_id: Option<UserId>,
}

impl ImportUrl {
    pub fn new(url: Url) -> Self {
        Self {
            url,
            filename: None,
            description: None,
            size: None,
            user_id: None,
        }
    }

    // TODO: builder pattern? .user_id(...).filename(...)
}

impl From<discord::Attachment> for ImportUrl {
    fn from(att: discord::Attachment) -> Self {
        Self {
            url: att.url.parse().unwrap(),
            filename: Some(att.filename),
            description: att.description,
            size: Some(att.size as u64),
            user_id: None,
        }
    }
}
