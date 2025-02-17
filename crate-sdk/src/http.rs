use anyhow::Result;
use reqwest::{StatusCode, Url};
use types::{
    Media, MediaCreate, MediaCreated, MediaId, Message, MessageCreateRequest, MessageId,
    MessagePatch, RoomId, SessionToken, Thread, ThreadCreateRequest, ThreadId, ThreadPatch,
};

const DEFAULT_BASE: &str = "https://chat.celery.eu.org/";

pub struct Http {
    token: SessionToken,
    base_url: Url,
}

impl Http {
    pub fn new(token: SessionToken) -> Self {
        let base_url = Url::parse(DEFAULT_BASE).unwrap();
        Self { token, base_url }
    }

    pub fn with_base_url(self, base_url: Url) -> Self {
        Self { base_url, ..self }
    }

    pub async fn message_create(
        &self,
        thread_id: ThreadId,
        body: &MessageCreateRequest,
    ) -> Result<Message> {
        let c = reqwest::Client::new();
        let url = self
            .base_url
            .join(&format!("/api/v1/thread/{thread_id}/message"))?;
        let res: Message = c
            .post(url)
            .bearer_auth(&self.token)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(res)
    }

    pub async fn message_update(
        &self,
        thread_id: ThreadId,
        message_id: MessageId,
        body: &MessagePatch,
    ) -> Result<Message> {
        let c = reqwest::Client::new();
        let url = self
            .base_url
            .join(&format!("/api/v1/thread/{thread_id}/message/{message_id}"))?;
        let res: Message = c
            .patch(url)
            .bearer_auth(&self.token)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(res)
    }

    pub async fn message_get(&self, thread_id: ThreadId, message_id: MessageId) -> Result<Message> {
        let c = reqwest::Client::new();
        let url = self
            .base_url
            .join(&format!("/api/v1/thread/{thread_id}/message/{message_id}"))?;
        let res: Message = c
            .get(url)
            .bearer_auth(&self.token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(res)
    }

    pub async fn message_delete(&self, thread_id: ThreadId, message_id: MessageId) -> Result<()> {
        let c = reqwest::Client::new();
        let url = self
            .base_url
            .join(&format!("/api/v1/thread/{thread_id}/message/{message_id}"))?;
        c.delete(url)
            .bearer_auth(&self.token)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn media_create(&self, body: &MediaCreate) -> Result<MediaCreated> {
        let c = reqwest::Client::new();
        let url = self.base_url.join("/api/v1/media")?;
        let res = c
            .post(url)
            .bearer_auth(&self.token)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(res)
    }

    pub async fn media_upload(
        &self,
        target: &MediaCreated,
        body: Vec<u8>,
    ) -> Result<Option<Media>> {
        let c = reqwest::Client::new();
        let res = c
            .patch(target.upload_url.clone().unwrap())
            .bearer_auth(&self.token)
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

    pub async fn thread_create(
        &self,
        room_id: RoomId,
        body: &ThreadCreateRequest,
    ) -> Result<Thread> {
        let c = reqwest::Client::new();
        let url = self
            .base_url
            .join(&format!("/api/v1/room/{room_id}/thread"))?;
        let res: Thread = c
            .post(url)
            .bearer_auth(&self.token)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(res)
    }

    pub async fn thread_update(&self, thread_id: ThreadId, body: &ThreadPatch) -> Result<Thread> {
        let c = reqwest::Client::new();
        let url = self.base_url.join(&format!("/api/v1/thread/{thread_id}"))?;
        let res: Thread = c
            .patch(url)
            .bearer_auth(&self.token)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(res)
    }

    pub async fn media_info_get(&self, media_id: MediaId) -> Result<Media> {
        let c = reqwest::Client::new();
        let url = self.base_url.join(&format!("/api/v1/media/{media_id}"))?;
        let res: Media = c
            .get(url)
            .bearer_auth(&self.token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(res)
    }
}
