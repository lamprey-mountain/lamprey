use anyhow::Result;
use headers::HeaderMapExt;
use reqwest::{header::HeaderMap, StatusCode, Url};
use types::{
    Media, MediaCreate, MediaCreated, MediaId, Message, MessageCreate, MessageId, MessagePatch,
    RoomId, SessionToken, Thread, ThreadCreate, ThreadId, ThreadPatch,
};

const DEFAULT_BASE: &str = "https://chat.celery.eu.org/";

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

    pub async fn media_upload(
        &self,
        target: &MediaCreated,
        body: Vec<u8>,
    ) -> Result<Option<Media>> {
        let res = self
            .client
            .patch(target.upload_url.clone().unwrap())
            .header("upload-offset", "0")
            .body(body)
            .send()
            .await?
            .error_for_status()?;
        match res.status() {
            StatusCode::OK => Ok(Some(res.json().await?)),
            StatusCode::NO_CONTENT => Ok(None),
            _ => unreachable!("technically reachable with a bad server"),
        }
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
                    .await?
                    .error_for_status()?
                    .json()
                    .await?;
                Ok(res)
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
                    .send()
                    .await?
                    .error_for_status()?
                    .json()
                    .await?;
                Ok(res)
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
                self.client
                    .$method(url)
                    .header("content-type", "application/json")
                    .json(body)
                    .send()
                    .await?
                    .error_for_status()?;
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
                self.client
                    .$method(url)
                    .send()
                    .await?
                    .error_for_status()?;
                Ok(())
            }
        }
    };
}

route!(get    "/api/v1/media/{media_id}"                        => media_info_get(media_id: MediaId) -> Media);
route!(post   "/api/v1/room/{room_id}/thread"                   => thread_create(room_id: RoomId) -> Thread, ThreadCreate);
route!(patch  "/api/v1/thread/{thread_id}"                      => thread_update(thread_id: ThreadId) -> Thread, ThreadPatch);
route!(post   "/api/v1/media"                                   => media_create() -> MediaCreated, MediaCreate);
route!(delete "/api/v1/thread/{thread_id}/message/{message_id}" => message_delete(thread_id: ThreadId, message_id: MessageId));
route!(patch  "/api/v1/thread/{thread_id}/message/{message_id}" => message_update(thread_id: ThreadId, message_id: MessageId) -> Message, MessagePatch);
route!(get    "/api/v1/thread/{thread_id}/message/{message_id}" => message_get(thread_id: ThreadId, message_id: MessageId) -> Message);
route!(post   "/api/v1/thread/{thread_id}/message"              => message_create(thread_id: ThreadId) -> Message, MessageCreate);
