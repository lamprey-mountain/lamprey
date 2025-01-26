use anyhow::Result;
use reqwest::Url;
use types::{Message, MessageCreateRequest, SessionToken, ThreadId};

const DEFAULT_BASE: &str = "https://chat.celery.eu.org/";

pub struct Http {
    // TODO: make private
    pub token: SessionToken,
    base_url: Url,
}

impl Http {
    pub fn new(token: SessionToken) -> Self {
        let base_url = Url::parse(DEFAULT_BASE).unwrap();
        Self { token, base_url }
    }
    
    pub async fn send_message(&self, thread_id: ThreadId, body: &MessageCreateRequest) -> Result<Message> {
        let c = reqwest::Client::new();
        let url = self.base_url.join(&format!("/api/v1/thread/{thread_id}/message"))?;
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
}
