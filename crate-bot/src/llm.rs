//! contains [`Llm`], a llm client

use crate::config::ConfigLlm;
use futures::{Stream, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use tokio_util::codec::FramedRead;
use tokio_util::codec::LinesCodec;
use tokio_util::io::StreamReader;

#[derive(Debug, Deserialize)]
pub struct ChatCompletionChunk {
    pub choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub delta: Delta,
}

#[derive(Debug, Deserialize)]
pub struct Delta {
    pub content: Option<String>,
}

/// a single message in an llm conversation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

/// the role of a [`ChatMessage`]
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

/// a client for an openai compatible api
pub struct Llm {
    http: reqwest::Client,
    config: ConfigLlm,
}

impl Llm {
    /// create a new client
    pub fn new(config: ConfigLlm) -> Self {
        Self {
            http: reqwest::Client::new(),
            config,
        }
    }

    /// generate a new chat completion, returning a stream of chunks
    pub async fn complete(
        &self,
        messages: &[ChatMessage],
    ) -> reqwest::Result<impl Stream<Item = ChatCompletionChunk>> {
        let response = self
            .http
            .post(format!("{}/chat/completions", self.config.base_url))
            .bearer_auth(&self.config.token)
            .json(&serde_json::json!({
                "model": self.config.model,
                "messages": messages,
                "stream": true,
            }))
            .send()
            .await?
            .error_for_status()?;

        let stream = response
            .bytes_stream()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));
        let reader = StreamReader::new(stream);
        let lines = FramedRead::new(reader, LinesCodec::new());

        Ok(lines.filter_map(|line| async move {
            let line = line.ok()?;
            let data = line.strip_prefix("data: ")?;
            if data == "[DONE]" {
                return None;
            }
            let parsed = serde_json::from_str::<ChatCompletionChunk>(data).ok()?;
            if parsed.text()?.is_empty() {
                return None;
            }
            Some(parsed)
        }))
    }
}

impl ChatCompletionChunk {
    /// the first content delta of this chunk, if any
    pub fn text(&self) -> Option<&str> {
        self.choices.first()?.delta.content.as_deref()
    }
}
