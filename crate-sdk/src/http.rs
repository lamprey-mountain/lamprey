use anyhow::{Context, Result};
use common::v1::types::pagination::{PaginationQuery, PaginationResponse};
use common::v1::types::{
    media::MediaCreated, misc::UserIdReq, user_status::StatusPatch, ApplicationId, Channel,
    ChannelCreate, ChannelId, ChannelPatch, ChannelReorder, Media, MediaCreate, MediaId, Message,
    MessageCreate, MessageId, MessageModerate, MessagePatch, MessageVerId, PinsReorder,
    PuppetCreate, Room, RoomBan, RoomBanBulkCreate, RoomCreate, RoomId, RoomPatch, SessionToken,
    ThreadMember, ThreadMemberPut, User, UserId, UserPatch, UserWithRelationship,
};
use common::v1::types::{
    MessageMigrate, RoomBanCreate, RoomMember, RoomMemberPatch, RoomMemberPut, SuspendRequest,
    TransferOwnership, UserCreate,
};
use headers::HeaderMapExt;
use reqwest::{header::HeaderMap, StatusCode, Url};
use serde_json::json;
use tracing::error;

const DEFAULT_BASE: &str = "https://chat.celery.eu.org/";

#[derive(Clone)]
pub struct Http {
    token: SessionToken,
    base_url: Url,
    client: reqwest::Client,
}

impl Http {
    pub fn new(token: SessionToken) -> Self {
        let base_url = Url::parse(DEFAULT_BASE).unwrap();
        let mut h = HeaderMap::new();
        h.typed_insert(headers::Authorization::bearer(&token.0).unwrap());
        let client = reqwest::Client::builder()
            .default_headers(h)
            .build()
            .unwrap();
        Self {
            token,
            base_url,
            client,
        }
    }

    pub fn with_base_url(self, base_url: Url) -> Self {
        let mut h = HeaderMap::new();
        h.typed_insert(headers::Authorization::bearer(&self.token.0).unwrap());
        let client = reqwest::Client::builder()
            .default_headers(h)
            .build()
            .unwrap();
        Self {
            base_url,
            client,
            ..self
        }
    }

    pub fn for_puppet(&self, id: UserId) -> Self {
        let mut h = HeaderMap::new();
        h.typed_insert(headers::Authorization::bearer(&self.token.0).unwrap());
        h.insert("x-puppet-id", id.to_string().try_into().unwrap());
        let client = reqwest::Client::builder()
            .default_headers(h)
            .build()
            .unwrap();
        Self {
            client,
            ..self.clone()
        }
    }

    pub async fn media_upload(
        &self,
        target: &MediaCreated,
        body: Vec<u8>,
    ) -> Result<Option<Media>> {
        let res = self
            .client
            .patch(target.upload_url.clone().unwrap())
            .header("upload-offset", "0")
            .header("content-type", "application/octet-stream")
            .header("content-length", body.len())
            .body(body)
            .send()
            .await?
            .error_for_status()?;
        match res.status() {
            StatusCode::OK => {
                let text = res.text().await?;
                serde_json::from_str(&text)
                    .with_context(|| {
                        error!(response_body = %text, "failed to decode media upload response body");
                        "failed to decode media upload response body"
                    })
                    .map(Some)
            }
            StatusCode::NO_CONTENT => Ok(None),
            _ => unreachable!("technically reachable with a bad server"),
        }
    }

    pub async fn thread_list(
        &self,
        channel_id: ChannelId,
        query: &PaginationQuery<ChannelId>,
    ) -> Result<PaginationResponse<Channel>> {
        let url = self
            .base_url
            .join(&format!("/api/v1/channel/{channel_id}/thread"))?;
        let res = self.client.get(url).query(query).send().await?;
        let res = res.error_for_status()?;
        let text = res.text().await?;
        serde_json::from_str(&text).with_context(|| {
            error!(response_body = %text, "failed to decode response body");
            "failed to decode response body for thread_list"
        })
    }

    pub async fn channel_list(
        &self,
        room_id: RoomId,
        query: &PaginationQuery<ChannelId>,
    ) -> Result<PaginationResponse<Channel>> {
        let url = self
            .base_url
            .join(&format!("/api/v1/room/{room_id}/channel"))?;
        let res = self.client.get(url).query(query).send().await?;
        let res = res.error_for_status()?;
        let text = res.text().await?;
        serde_json::from_str(&text).with_context(|| {
            error!(response_body = %text, "failed to decode response body");
            "failed to decode response body for thread_list"
        })
    }

    pub async fn message_list(
        &self,
        channel_id: ChannelId,
        query: &PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>> {
        let url = self
            .base_url
            .join(&format!("/api/v1/channel/{channel_id}/message"))?;
        let res = self.client.get(url).query(query).send().await?;
        let res = res.error_for_status()?;
        let text = res.text().await?;
        serde_json::from_str(&text).with_context(|| {
            error!(response_body = %text, "failed to decode response body");
            "failed to decode response body for message_list"
        })
    }
}

macro_rules! route {
    ($method: ident $url:expr => $name:ident($($param:ident: $param_type:ty),*) -> $res:ty, $req:ty) => {
        impl Http {
            pub async fn $name(
                &self,
                $($param: $param_type,)*
                body: &$req,
            ) -> Result<$res> {
                let url = self.base_url.join(&format!($url))?;
                let res = self.client
                    .$method(url)
                    .header("content-type", "application/json")
                    .json(body)
                    .send()
                    .await?;
                let res = res.error_for_status()?;
                let text = res.text().await?;
                serde_json::from_str(&text).with_context(|| {
                    error!(response_body = %text, "failed to decode response body");
                    format!("failed to decode response body for {}", stringify!($name))
                })
            }
        }
    };

    ($method: ident $url:expr => $name:ident($($param:ident: $param_type:ty),*) -> $res:ty) => {
        impl Http {
            pub async fn $name(
                &self,
                $($param: $param_type),*
            ) -> Result<$res> {
                let url = self.base_url.join(&format!($url))?;
                let res = self.client
                    .$method(url)
                    .header("content-type", "application/json")
                    .json(&json!({}))
                    .send()
                    .await?;
                let res = res.error_for_status()?;
                let text = res.text().await?;
                serde_json::from_str(&text).with_context(|| {
                    error!(response_body = %text, "failed to decode response body");
                    format!("failed to decode response body for {}", stringify!($name))
                })
            }
        }
    };

    ($method: ident $url:expr => $name:ident($($param:ident: $param_type:ty),*), $req:ty) => {
        impl Http {
            pub async fn $name(
                &self,
                $($param: $param_type),*,
                body: &$req,
            ) -> Result<()> {
                let url = self.base_url.join(&format!($url))?;
                let res = self.client
                    .$method(url)
                    .header("content-type", "application/json")
                    .json(body)
                    .send()
                    .await?;
                if let Err(e) = res.error_for_status_ref() {
                    let text = res.text().await.unwrap_or_else(|_| "failed to read body".to_string());
                    error!(name = stringify!($name), status = %e.status().unwrap(), response_body = %text, "request failed");
                    return Err(anyhow::anyhow!(e).context(text));
                }
                Ok(())
            }
        }
    };

    ($method: ident $url:expr => $name:ident($($param:ident: $param_type:ty),*)) => {
        impl Http {
            pub async fn $name(
                &self,
                $($param: $param_type),*,
            ) -> Result<()> {
                let url = self.base_url.join(&format!($url))?;
                let res = self.client
                    .$method(url)
                    .header("content-type", "application/json")
                    .json(&json!({}))
                    .send()
                    .await?;
                if let Err(e) = res.error_for_status_ref() {
                    let text = res.text().await.unwrap_or_else(|_| "failed to read body".to_string());
                    error!(name = stringify!($name), status = %e.status().unwrap(), response_body = %text, "request failed");
                    return Err(anyhow::anyhow!(e).context(text));
                }
                Ok(())
            }
        }
    };
}

route!(get    "/api/v1/media/{media_id}"                          => media_info_get(media_id: MediaId) -> Media);
route!(post   "/api/v1/room/{room_id}/channel"                    => channel_create_room(room_id: RoomId) -> Channel, ChannelCreate);
route!(patch  "/api/v1/channel/{channel_id}"                      => channel_update(channel_id: ChannelId) -> Channel, ChannelPatch);
route!(get    "/api/v1/channel/{channel_id}"                      => channel_get(channel_id: ChannelId) -> Channel);
route!(post   "/api/v1/media"                                     => media_create() -> MediaCreated, MediaCreate);
route!(delete "/api/v1/channel/{channel_id}/message/{message_id}" => message_delete(channel_id: ChannelId, message_id: MessageId));
route!(patch  "/api/v1/channel/{channel_id}/message/{message_id}" => message_edit(channel_id: ChannelId, message_id: MessageId) -> Message, MessagePatch);
route!(get    "/api/v1/channel/{channel_id}/message/{message_id}" => message_get(channel_id: ChannelId, message_id: MessageId) -> Message);
route!(post   "/api/v1/channel/{channel_id}/message"              => message_create(channel_id: ChannelId) -> Message, MessageCreate);
route!(put    "/api/v1/channel/{channel_id}/message/{message_id}/reaction/{reaction}" => message_react(channel_id: ChannelId, message_id: MessageId, reaction: String));
route!(delete "/api/v1/channel/{channel_id}/message/{message_id}/reaction/{reaction}" => message_unreact(channel_id: ChannelId, message_id: MessageId, reaction: String));
route!(post   "/api/v1/channel/{channel_id}/typing"               => channel_typing(channel_id: ChannelId));
route!(get    "/api/v1/user/{user_id}"                            => user_get(user_id: UserIdReq) -> UserWithRelationship);
route!(put    "/api/v1/room/{room_id}/member/{user_id}"           => room_member_add(room_id: RoomId, user_id: UserIdReq) -> RoomMember, RoomMemberPut);
route!(patch  "/api/v1/room/{room_id}/member/{user_id}"           => room_member_patch(room_id: RoomId, user_id: UserIdReq) -> RoomMember, RoomMemberPatch);
// route!(post   "/api/v1/user"                                      => user_create() -> User, UserCreate);
route!(patch  "/api/v1/user/{user_id}"                            => user_update(user_id: UserIdReq) -> User, UserPatch);
route!(post   "/api/v1/user/{user_id}/status"                     => user_set_status(user_id: UserIdReq), StatusPatch);
route!(put    "/api/v1/app/{app_id}/puppet/{puppet_id}"           => puppet_ensure(app_id: ApplicationId, puppet_id: String) -> User, PuppetCreate);
route!(post   "/api/v1/channel"                                   => channel_create_dm() -> Channel, ChannelCreate);
route!(patch  "/api/v1/room/{room_id}/channel"                    => channel_reorder(room_id: RoomId), ChannelReorder);
route!(put    "/api/v1/channel/{channel_id}/remove"               => channel_remove(channel_id: ChannelId));
route!(delete "/api/v1/channel/{channel_id}/remove"               => channel_restore(channel_id: ChannelId));
route!(post   "/api/v1/channel/{channel_id}/upgrade"              => channel_upgrade(channel_id: ChannelId) -> Room);
route!(post   "/api/v1/channel/{channel_id}/transfer-ownership"   => channel_transfer_ownership(channel_id: ChannelId), TransferOwnership);
route!(post   "/api/v1/user/@self/dm/{target_id}"                 => dm_init(target_id: UserId) -> Channel);
route!(get    "/api/v1/user/@self/dm/{target_id}"                 => dm_get(target_id: UserId) -> Channel);
route!(get    "/api/v1/channel/{channel_id}/message/{message_id}/version/{version_id}" => message_version_get(channel_id: ChannelId, message_id: MessageId, version_id: MessageVerId) -> Message);
route!(patch  "/api/v1/channel/{channel_id}/message"              => message_moderate(channel_id: ChannelId), MessageModerate);
route!(post   "/api/v1/channel/{channel_id}/move-messages"        => message_move(channel_id: ChannelId), MessageMigrate);
route!(put    "/api/v1/channel/{channel_id}/pin/{message_id}"     => message_pin_create(channel_id: ChannelId, message_id: MessageId));
route!(delete "/api/v1/channel/{channel_id}/pin/{message_id}"     => message_pin_delete(channel_id: ChannelId, message_id: MessageId));
route!(patch  "/api/v1/channel/{channel_id}/pin"                  => message_pin_reorder(channel_id: ChannelId), PinsReorder);
route!(post   "/api/v1/room"                                      => room_create() -> Room, RoomCreate);
route!(get    "/api/v1/room/{room_id}"                            => room_get(room_id: RoomId) -> Room);
route!(patch  "/api/v1/room/{room_id}"                            => room_edit(room_id: RoomId) -> Room, RoomPatch);
route!(delete "/api/v1/room/{room_id}"                            => room_delete(room_id: RoomId));
route!(post   "/api/v1/room/{room_id}/undelete"                   => room_undelete(room_id: RoomId));
route!(put    "/api/v1/room/{room_id}/ack"                        => room_ack(room_id: RoomId));
route!(post   "/api/v1/room/{room_id}/quarantine"                 => room_quarantine(room_id: RoomId) -> Room);
route!(delete "/api/v1/room/{room_id}/quarantine"                 => room_unquarantine(room_id: RoomId) -> Room);
route!(post   "/api/v1/room/{room_id}/transfer-ownership"         => room_transfer_ownership(room_id: RoomId), TransferOwnership);
route!(get    "/api/v1/room/{room_id}/member/{user_id}"           => room_member_get(room_id: RoomId, user_id: UserIdReq) -> RoomMember);
route!(delete "/api/v1/room/{room_id}/member/{user_id}"           => room_member_delete(room_id: RoomId, user_id: UserIdReq));
route!(put    "/api/v1/room/{room_id}/ban/{user_id}"              => room_ban_create(room_id: RoomId, user_id: UserIdReq), RoomBanCreate);
route!(post   "/api/v1/room/{room_id}/ban"                        => room_ban_create_bulk(room_id: RoomId), RoomBanBulkCreate);
route!(delete "/api/v1/room/{room_id}/ban/{user_id}"              => room_ban_remove(room_id: RoomId, user_id: UserIdReq));
route!(get    "/api/v1/room/{room_id}/ban/{user_id}"              => room_ban_get(room_id: RoomId, user_id: UserIdReq) -> RoomBan);
route!(get    "/api/v1/thread/{thread_id}/member/{user_id}"       => thread_member_get(thread_id: ChannelId, user_id: UserIdReq) -> ThreadMember);
route!(put    "/api/v1/thread/{thread_id}/member/{user_id}"       => thread_member_add(thread_id: ChannelId, user_id: UserIdReq) -> ThreadMember, ThreadMemberPut);
route!(delete "/api/v1/thread/{thread_id}/member/{user_id}"       => thread_member_delete(thread_id: ChannelId, user_id: UserIdReq));
route!(delete "/api/v1/user/{user_id}"                            => user_delete(user_id: UserIdReq));
route!(post   "/api/v1/user/{user_id}/undelete"                   => user_undelete(user_id: UserIdReq));
route!(post   "/api/v1/guest"                                     => guest_create() -> User, UserCreate);
route!(post   "/api/v1/user/{user_id}/suspend"                    => user_suspend(user_id: UserIdReq) -> User, SuspendRequest);
route!(delete "/api/v1/user/{user_id}/suspend"                    => user_unsuspend(user_id: UserIdReq) -> User);
