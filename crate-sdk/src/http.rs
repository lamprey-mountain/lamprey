use anyhow::{Context, Result};
use common::v1::types::pagination::{PaginationQuery, PaginationResponse};
use common::v1::types::{
    media::MediaCreated, misc::UserIdReq, user_status::StatusPatch, ApplicationId, Media,
    MediaCreate, MediaId, Message, MessageCreate, MessageId, MessagePatch, PuppetCreate, RoomId,
    SessionToken, Thread, ThreadCreate, ThreadId, ThreadPatch, User, UserId, UserPatch,
};
use common::v1::types::{RoomMember, RoomMemberPatch};
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
        room_id: RoomId,
        query: &PaginationQuery<ThreadId>,
    ) -> Result<PaginationResponse<Thread>> {
        let url = self
            .base_url
            .join(&format!("/api/v1/room/{room_id}/thread"))?;
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
        thread_id: ThreadId,
        query: &PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>> {
        let url = self
            .base_url
            .join(&format!("/api/v1/thread/{thread_id}/message"))?;
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

// FIXME: 304 not modified (see room_member.rs)
route!(get    "/api/v1/media/{media_id}"                        => media_info_get(media_id: MediaId) -> Media);
route!(post   "/api/v1/room/{room_id}/thread"                   => thread_create(room_id: RoomId) -> Thread, ThreadCreate);
route!(patch  "/api/v1/thread/{thread_id}"                      => thread_update(thread_id: ThreadId) -> Thread, ThreadPatch);
route!(post   "/api/v1/media"                                   => media_create() -> MediaCreated, MediaCreate);
route!(delete "/api/v1/thread/{thread_id}/message/{message_id}" => message_delete(thread_id: ThreadId, message_id: MessageId));
route!(patch  "/api/v1/thread/{thread_id}/message/{message_id}" => message_update(thread_id: ThreadId, message_id: MessageId) -> Message, MessagePatch);
route!(get    "/api/v1/thread/{thread_id}/message/{message_id}" => message_get(thread_id: ThreadId, message_id: MessageId) -> Message);
route!(post   "/api/v1/thread/{thread_id}/message"              => message_create(thread_id: ThreadId) -> Message, MessageCreate);
route!(put    "/api/v1/thread/{thread_id}/message/{message_id}/reaction/{reaction}" => message_react(thread_id: ThreadId, message_id: MessageId, reaction: String));
route!(delete "/api/v1/thread/{thread_id}/message/{message_id}/reaction/{reaction}" => message_unreact(thread_id: ThreadId, message_id: MessageId, reaction: String));
route!(post   "/api/v1/thread/{thread_id}/typing"               => typing_start(thread_id: ThreadId));
route!(get    "/api/v1/user/{user_id}"                          => user_get(user_id: UserId) -> User);
route!(put    "/api/v1/room/{room_id}/member/{user_id}"         => room_member_put(room_id: RoomId, user_id: UserId));
route!(patch  "/api/v1/room/{room_id}/member/{user_id}"         => room_member_patch(room_id: RoomId, user_id: UserIdReq) -> RoomMember, RoomMemberPatch);
// route!(post   "/api/v1/user"                                    => user_create() -> User, UserCreate);
route!(patch  "/api/v1/user/{user_id}"                          => user_update(user_id: UserIdReq) -> User, UserPatch);
route!(post   "/api/v1/user/{user_id}/status"                   => user_set_status(user_id: UserIdReq), StatusPatch);
route!(put    "/api/v1/app/{app_id}/puppet/{puppet_id}"         => puppet_ensure(app_id: ApplicationId, puppet_id: String) -> User, PuppetCreate);
