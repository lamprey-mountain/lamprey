use anyhow::{Context, Result};
use common::v1::types::pagination::{PaginationQuery, PaginationResponse};
use common::v1::types::presence::Presence;
use common::v1::types::util::Time;
use common::v1::types::{
    emoji::{EmojiCustom, EmojiCustomCreate, EmojiCustomPatch, EmojiSearchQuery},
    misc::UserIdReq,
    reaction::{ReactionKeyParam, ReactionListItem},
    role::RoleDeleteQuery,
    ApplicationId, Channel, ChannelCreate, ChannelId, ChannelPatch, ChannelReorder, EmojiId,
    Invite, InviteCode, InviteCreate, InvitePatch, MediaId, MessageId, MessageModerate,
    MessagePatch, MessageVerId, PermissionOverwriteSet, PinsReorder, PuppetCreate, Role,
    RoleCreate, RoleId, RoleMemberBulkPatch, RolePatch, RoleReorder, Room, RoomBan,
    RoomBanBulkCreate, RoomCreate, RoomId, RoomMember, RoomMemberPatch, RoomMemberPut, RoomPatch,
    SessionToken, ThreadMember, ThreadMemberPut, User, UserId, UserPatch, UserWithRelationship,
};
use common::v1::types::{
    MessageCreate, MessageMigrate, RoomBanCreate, SuspendRequest, TransferOwnership, UserCreate,
};
use common::v2::types::media::{Media, MediaCreate, MediaCreated, MediaDoneParams};
use common::v2::types::message::Message;
use headers::HeaderMapExt;
use reqwest::{header::HeaderMap, StatusCode, Url};
use serde_json::json;
use tracing::error;
use uuid::Uuid;

use crate::consts::DEFAULT_API_URL;

#[derive(Clone)]
pub struct Http {
    token: SessionToken,
    base_url: Url,
    client: reqwest::Client,
}

impl Http {
    pub fn new(token: SessionToken) -> Self {
        let base_url = Url::parse(DEFAULT_API_URL).unwrap();
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
                    .$method(url.clone())
                    .header("content-type", "application/json")
                    .json(body)
                    .send()
                    .await?;
                let status = res.status();
                let text = res.text().await.unwrap_or_else(|_| "failed to read body".to_string());
                if !status.is_success() {
                    error!(name = stringify!($name), status = %status, response_body = %text, url = %url, "request failed");
                    return Err(anyhow::anyhow!("request failed with status {}: {}", status, text));
                }
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
                    .$method(url.clone())
                    .header("content-type", "application/json")
                    .json(&json!({}))
                    .send()
                    .await?;
                let status = res.status();
                let text = res.text().await.unwrap_or_else(|_| "failed to read body".to_string());
                if !status.is_success() {
                    error!(name = stringify!($name), status = %status, response_body = %text, url = %url, "request failed");
                    return Err(anyhow::anyhow!("request failed with status {}: {}", status, text));
                }
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

    // GET with query parameters
    ($method: ident $url:expr => $name:ident($($param:ident: $param_type:ty),* , $query_param:ident: $query_type:ty) -> $res:ty) => {
        impl Http {
            pub async fn $name(
                &self,
                $($param: $param_type,)*
                $query_param: $query_type,
            ) -> Result<$res> {
                let url = self.base_url.join(&format!($url))?;
                let res = self.client
                    .$method(url.clone())
                    .query(&$query_param)
                    .send()
                    .await?;
                let status = res.status();
                let text = res.text().await.unwrap_or_else(|_| "failed to read body".to_string());
                if !status.is_success() {
                    error!(name = stringify!($name), status = %status, response_body = %text, url = %url, "request failed");
                    return Err(anyhow::anyhow!("request failed with status {}: {}", status, text));
                }
                serde_json::from_str(&text).with_context(|| {
                    error!(response_body = %text, "failed to decode response body");
                    format!("failed to decode response body for {}", stringify!($name))
                })
            }
        }
    };

    // GET with multiple query parameters
    ($method: ident $url:expr => $name:ident($($param:ident: $param_type:ty),* , $query_param1:ident: $query_type1:ty, $query_param2:ident: $query_type2:ty) -> $res:ty) => {
        impl Http {
            pub async fn $name(
                &self,
                $($param: $param_type,)*
                $query_param1: $query_type1,
                $query_param2: $query_type2,
            ) -> Result<$res> {
                let url = self.base_url.join(&format!($url))?;
                let res = self.client
                    .$method(url.clone())
                    .query(&$query_param1)
                    .query(&$query_param2)
                    .send()
                    .await?;
                let status = res.status();
                let text = res.text().await.unwrap_or_else(|_| "failed to read body".to_string());
                if !status.is_success() {
                    error!(name = stringify!($name), status = %status, response_body = %text, url = %url, "request failed");
                    return Err(anyhow::anyhow!("request failed with status {}: {}", status, text));
                }
                serde_json::from_str(&text).with_context(|| {
                    error!(response_body = %text, "failed to decode response body");
                    format!("failed to decode response body for {}", stringify!($name))
                })
            }
        }
    };

    // DELETE/PATCH with query parameters (returns ())
    ($method: ident $url:expr => $name:ident($($param:ident: $param_type:ty),* , $query_param:ident: $query_type:ty)) => {
        impl Http {
            pub async fn $name(
                &self,
                $($param: $param_type,)*
                $query_param: $query_type,
            ) -> Result<()> {
                let url = self.base_url.join(&format!($url))?;
                let res = self.client
                    .$method(url)
                    .query(&$query_param)
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

// Media Routes
route!(get    "/api/v1/media/{media_id}"                          => media_info_get(media_id: MediaId) -> Media);
route!(post   "/api/v1/media"                                     => media_create() -> MediaCreated, MediaCreate);
route!(put    "/api/v1/media/{media_id}/done"                     => media_done(media_id: MediaId) -> Option<Media>, MediaDoneParams);

// Channel Routes
route!(post   "/api/v1/room/{room_id}/channel"                    => channel_create_room(room_id: RoomId) -> Channel, ChannelCreate);
route!(post   "/api/v1/channel"                                   => channel_create_dm() -> Channel, ChannelCreate);
route!(get    "/api/v1/channel/{channel_id}"                      => channel_get(channel_id: ChannelId) -> Channel);
route!(patch  "/api/v1/channel/{channel_id}"                      => channel_update(channel_id: ChannelId) -> Channel, ChannelPatch);
route!(patch  "/api/v1/room/{room_id}/channel"                    => channel_reorder(room_id: RoomId), ChannelReorder);
route!(put    "/api/v1/channel/{channel_id}/remove"               => channel_remove(channel_id: ChannelId));
route!(delete "/api/v1/channel/{channel_id}/remove"               => channel_restore(channel_id: ChannelId));
route!(post   "/api/v1/channel/{channel_id}/upgrade"              => channel_upgrade(channel_id: ChannelId) -> Room);
route!(post   "/api/v1/channel/{channel_id}/transfer-ownership"   => channel_transfer_ownership(channel_id: ChannelId), TransferOwnership);
route!(post   "/api/v1/channel/{channel_id}/typing"               => channel_typing(channel_id: ChannelId));

// Message Routes
route!(post   "/api/v1/channel/{channel_id}/message"              => message_create(channel_id: ChannelId) -> Message, MessageCreate);
route!(get    "/api/v1/channel/{channel_id}/message/{message_id}" => message_get(channel_id: ChannelId, message_id: MessageId) -> Message);
route!(patch  "/api/v1/channel/{channel_id}/message/{message_id}" => message_edit(channel_id: ChannelId, message_id: MessageId) -> Message, MessagePatch);
route!(delete "/api/v1/channel/{channel_id}/message/{message_id}" => message_delete(channel_id: ChannelId, message_id: MessageId));
route!(get    "/api/v1/channel/{channel_id}/message/{message_id}/version/{version_id}" => message_version_get(channel_id: ChannelId, message_id: MessageId, version_id: MessageVerId) -> Message);

// Message Reaction Routes (Basic)
route!(put    "/api/v1/channel/{channel_id}/message/{message_id}/reaction/{reaction}" => message_react(channel_id: ChannelId, message_id: MessageId, reaction: String));
route!(delete "/api/v1/channel/{channel_id}/message/{message_id}/reaction/{reaction}" => message_unreact(channel_id: ChannelId, message_id: MessageId, reaction: String));

// Message Pin Routes
route!(put    "/api/v1/channel/{channel_id}/pin/{message_id}"     => message_pin_create(channel_id: ChannelId, message_id: MessageId));
route!(delete "/api/v1/channel/{channel_id}/pin/{message_id}"     => message_pin_delete(channel_id: ChannelId, message_id: MessageId));
route!(patch  "/api/v1/channel/{channel_id}/pin"                  => message_pin_reorder(channel_id: ChannelId), PinsReorder);

// Message Moderate/Move Routes
route!(patch  "/api/v1/channel/{channel_id}/message"              => message_moderate(channel_id: ChannelId), MessageModerate);
route!(post   "/api/v1/channel/{channel_id}/move-messages"        => message_move(channel_id: ChannelId), MessageMigrate);

// Room Routes
route!(post   "/api/v1/room"                                      => room_create() -> Room, RoomCreate);
route!(get    "/api/v1/room/{room_id}"                            => room_get(room_id: RoomId) -> Room);
route!(patch  "/api/v1/room/{room_id}"                            => room_edit(room_id: RoomId) -> Room, RoomPatch);
route!(delete "/api/v1/room/{room_id}"                            => room_delete(room_id: RoomId));
route!(post   "/api/v1/room/{room_id}/undelete"                   => room_undelete(room_id: RoomId));
route!(put    "/api/v1/room/{room_id}/ack"                        => room_ack(room_id: RoomId));
route!(post   "/api/v1/room/{room_id}/quarantine"                 => room_quarantine(room_id: RoomId) -> Room);
route!(delete "/api/v1/room/{room_id}/quarantine"                 => room_unquarantine(room_id: RoomId) -> Room);
route!(post   "/api/v1/room/{room_id}/transfer-ownership"         => room_transfer_ownership(room_id: RoomId), TransferOwnership);

// Room Member Routes
route!(get    "/api/v1/room/{room_id}/member/{user_id}"           => room_member_get(room_id: RoomId, user_id: UserIdReq) -> RoomMember);
route!(put    "/api/v1/room/{room_id}/member/{user_id}"           => room_member_add(room_id: RoomId, user_id: UserIdReq) -> RoomMember, RoomMemberPut);
route!(patch  "/api/v1/room/{room_id}/member/{user_id}"           => room_member_patch(room_id: RoomId, user_id: UserIdReq) -> RoomMember, RoomMemberPatch);
route!(delete "/api/v1/room/{room_id}/member/{user_id}"           => room_member_delete(room_id: RoomId, user_id: UserIdReq));

// Room Ban Routes
route!(get    "/api/v1/room/{room_id}/ban/{user_id}"              => room_ban_get(room_id: RoomId, user_id: UserIdReq) -> RoomBan);
route!(put    "/api/v1/room/{room_id}/ban/{user_id}"              => room_ban_create(room_id: RoomId, user_id: UserIdReq), RoomBanCreate);
route!(post   "/api/v1/room/{room_id}/ban"                        => room_ban_create_bulk(room_id: RoomId), RoomBanBulkCreate);
route!(delete "/api/v1/room/{room_id}/ban/{user_id}"              => room_ban_remove(room_id: RoomId, user_id: UserIdReq));

// Thread Member Routes
route!(get    "/api/v1/thread/{thread_id}/member/{user_id}"       => thread_member_get(thread_id: ChannelId, user_id: UserIdReq) -> ThreadMember);
route!(put    "/api/v1/thread/{thread_id}/member/{user_id}"       => thread_member_add(thread_id: ChannelId, user_id: UserIdReq) -> ThreadMember, ThreadMemberPut);
route!(delete "/api/v1/thread/{thread_id}/member/{user_id}"       => thread_member_delete(thread_id: ChannelId, user_id: UserIdReq));

// User Routes
route!(get    "/api/v1/user/{user_id}"                            => user_get(user_id: UserIdReq) -> UserWithRelationship);
route!(patch  "/api/v1/user/{user_id}"                            => user_update(user_id: UserIdReq) -> User, UserPatch);
route!(post   "/api/v1/user/{user_id}/presence"                   => user_set_presence(user_id: UserIdReq), Presence);
route!(delete "/api/v1/user/{user_id}"                            => user_delete(user_id: UserIdReq));
route!(post   "/api/v1/user/{user_id}/undelete"                   => user_undelete(user_id: UserIdReq));
route!(post   "/api/v1/user/{user_id}/suspend"                    => user_suspend(user_id: UserIdReq) -> User, SuspendRequest);
route!(delete "/api/v1/user/{user_id}/suspend"                    => user_unsuspend(user_id: UserIdReq) -> User);
route!(post   "/api/v1/guest"                                     => guest_create() -> User, UserCreate);

// DM Routes
route!(post   "/api/v1/user/@self/dm/{target_id}"                 => dm_init(target_id: UserId) -> Channel);
route!(get    "/api/v1/user/@self/dm/{target_id}"                 => dm_get(target_id: UserId) -> Channel);

// Puppet/App Routes
route!(put    "/api/v1/app/{app_id}/puppet/{puppet_id}"           => puppet_ensure(app_id: ApplicationId, puppet_id: String) -> User, PuppetCreate);

// Emoji Routes
route!(post   "/api/v1/room/{room_id}/emoji"                      => emoji_create(room_id: RoomId) -> EmojiCustom, EmojiCustomCreate);
route!(get    "/api/v1/room/{room_id}/emoji/{emoji_id}"           => emoji_get(room_id: RoomId, emoji_id: EmojiId) -> EmojiCustom);
route!(delete "/api/v1/room/{room_id}/emoji/{emoji_id}"           => emoji_delete(room_id: RoomId, emoji_id: EmojiId));
route!(patch  "/api/v1/room/{room_id}/emoji/{emoji_id}"           => emoji_update(room_id: RoomId, emoji_id: EmojiId) -> EmojiCustom, EmojiCustomPatch);
route!(get    "/api/v1/room/{room_id}/emoji"                      => emoji_list(room_id: RoomId, _q: PaginationQuery<EmojiId>) -> PaginationResponse<EmojiCustom>);
route!(get    "/api/v1/emoji/{emoji_id}"                          => emoji_lookup(emoji_id: EmojiId) -> EmojiCustom);
route!(get    "/api/v1/emoji/search"                              => emoji_search(_q: EmojiSearchQuery, _pagination: PaginationQuery<EmojiId>) -> PaginationResponse<EmojiCustom>);

// Role Routes
route!(post   "/api/v1/room/{room_id}/role"                       => role_create(room_id: RoomId) -> Role, RoleCreate);
route!(patch  "/api/v1/room/{room_id}/role/{role_id}"             => role_update(room_id: RoomId, role_id: RoleId) -> Role, RolePatch);
route!(delete "/api/v1/room/{room_id}/role/{role_id}"             => role_delete(room_id: RoomId, role_id: RoleId, _query: RoleDeleteQuery));
route!(get    "/api/v1/room/{room_id}/role/{role_id}"             => role_get(room_id: RoomId, role_id: RoleId) -> Role);
route!(get    "/api/v1/room/{room_id}/role"                       => role_list(room_id: RoomId, _paginate: PaginationQuery<RoleId>) -> PaginationResponse<Role>);
route!(patch  "/api/v1/room/{room_id}/role"                       => role_reorder(room_id: RoomId), RoleReorder);
route!(get    "/api/v1/room/{room_id}/role/{role_id}/member"      => role_member_list(room_id: RoomId, role_id: RoleId, _paginate: PaginationQuery<UserId>) -> PaginationResponse<RoomMember>);
route!(put    "/api/v1/room/{room_id}/role/{role_id}/member/{user_id}" => role_member_add(room_id: RoomId, role_id: RoleId, user_id: UserId) -> RoomMember);
route!(delete "/api/v1/room/{room_id}/role/{role_id}/member/{user_id}" => role_member_remove(room_id: RoomId, role_id: RoleId, user_id: UserId) -> RoomMember);
route!(patch  "/api/v1/room/{room_id}/role/{role_id}/member"      => role_member_bulk_edit(room_id: RoomId, role_id: RoleId), RoleMemberBulkPatch);

// Reaction Routes (Additional)
route!(get    "/api/v1/channel/{channel_id}/message/{message_id}/reaction/{reaction_key}" => reaction_list(channel_id: ChannelId, message_id: MessageId, reaction_key: ReactionKeyParam, _q: PaginationQuery<UserId>) -> PaginationResponse<ReactionListItem>);
route!(delete "/api/v1/channel/{channel_id}/message/{message_id}/reaction/{reaction_key}" => reaction_remove_key(channel_id: ChannelId, message_id: MessageId, reaction_key: ReactionKeyParam));
route!(delete "/api/v1/channel/{channel_id}/message/{message_id}/reaction" => reaction_remove_all(channel_id: ChannelId, message_id: MessageId));

// Permission Overwrite Routes
route!(put    "/api/v1/channel/{channel_id}/permission/{overwrite_id}" => permission_overwrite_set(channel_id: ChannelId, overwrite_id: Uuid), PermissionOverwriteSet);
route!(delete "/api/v1/channel/{channel_id}/permission/{overwrite_id}" => permission_overwrite_delete(channel_id: ChannelId, overwrite_id: Uuid));

// Invite Routes
route!(delete "/api/v1/invite/{code}"                             => invite_delete(code: InviteCode));
route!(get    "/api/v1/invite/{code}"                             => invite_resolve(code: InviteCode) -> Invite);
route!(post   "/api/v1/invite/{code}"                             => invite_use(code: InviteCode));
route!(patch  "/api/v1/invite/{code}"                             => invite_patch(code: InviteCode), InvitePatch);
route!(post   "/api/v1/room/{room_id}/invite"                     => invite_room_create(room_id: RoomId) -> Invite, InviteCreate);
route!(get    "/api/v1/room/{room_id}/invite"                     => invite_room_list(room_id: RoomId, _q: PaginationQuery<InviteCode>) -> PaginationResponse<Invite>);
route!(post   "/api/v1/room/{room_id}/channel/{channel_id}/invite" => invite_channel_create(room_id: RoomId, channel_id: ChannelId) -> Invite, InviteCreate);
route!(get    "/api/v1/room/{room_id}/channel/{channel_id}/invite" => invite_channel_list(room_id: RoomId, channel_id: ChannelId, _q: PaginationQuery<InviteCode>) -> PaginationResponse<Invite>);
route!(post   "/api/v1/invite/server"                             => invite_server_create() -> Invite, InviteCreate);
route!(get    "/api/v1/invite/server"                             => invite_server_list(_q: PaginationQuery<InviteCode>) -> PaginationResponse<Invite>);
route!(post   "/api/v1/invite/user/{user_id}"                     => invite_user_create(user_id: UserId) -> Invite, InviteCreate);
route!(get    "/api/v1/invite/user/{user_id}"                     => invite_user_list(user_id: UserId, _q: PaginationQuery<InviteCode>) -> PaginationResponse<Invite>);

impl Http {
    /// Create a message with a custom timestamp (for bridge sync)
    pub async fn message_create_with_timestamp(
        &self,
        channel_id: ChannelId,
        body: &MessageCreate,
        timestamp: Time,
    ) -> Result<Message> {
        let url = self
            .base_url
            .join(&format!("/api/v1/channel/{}/message", channel_id))?;
        let req = self
            .client
            .post(url)
            .header("content-type", "application/json")
            .header("X-Timestamp", timestamp.unix_timestamp().to_string())
            .json(body);

        let res = req.send().await?;
        let res = res.error_for_status()?;
        let text = res.text().await?;
        serde_json::from_str(&text).with_context(|| {
            error!(response_body = %text, "failed to decode response body");
            "failed to decode response body for message_create_with_timestamp".to_string()
        })
    }
}
