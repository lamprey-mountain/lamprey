use crate::bot::Bot;
use crate::config::ConfigLlm;
use common::v1::types::Message;
use common::v1::types::MessageType;
use common::v1::types::components::{
    Component, ComponentCreate, ComponentId, ComponentType, Components,
};
use common::v1::types::flume::{FlumeAppendCreate, FlumeDeltaCreate, FlumeReplaceCreate};
use futures::{Stream, StreamExt, TryStreamExt};
use serde::Deserialize;
use tokio_util::codec::FramedRead;
use tokio_util::codec::LinesCodec;
use tokio_util::io::StreamReader;

#[derive(Deserialize, Debug)]
pub struct ChatCompletionChunk {
    choices: Vec<Choice>,
}

impl ChatCompletionChunk {
    /// the first content delta of this chunk, if any
    pub fn text(&self) -> Option<&str> {
        self.choices.first()?.delta.content.as_deref()
    }
}

#[derive(Deserialize, Debug)]
struct Choice {
    delta: Delta,
}

#[derive(Deserialize, Debug)]
struct Delta {
    content: Option<String>,
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
        prompt: String,
    ) -> reqwest::Result<impl Stream<Item = ChatCompletionChunk>> {
        let response = self
            .http
            .post(format!("{}/v1/chat/completions", self.config.base_url))
            .bearer_auth(&self.config.token)
            .json(&serde_json::json!({
                "model": self.config.model,
                "messages": [{"role": "user", "content": prompt}],
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
            parsed.text()?;
            Some(parsed)
        }))
    }
}

impl Bot {
    pub(crate) fn handle_llm(&self, message: &Message) {
        let Some(llm_config) = &self.config.llm else {
            return;
        };

        let Some(current_user_id) = self.user_id else {
            return;
        };

        if message.author_id == current_user_id {
            return;
        }

        if !llm_config.channel_ids.contains(&message.channel_id) {
            return;
        }

        let content = match &message.latest_version.message_type {
            MessageType::DefaultMarkdown(m) | MessageType::ThreadInitial(m) => m.content.as_deref(),
            _ => None,
        };

        let Some(prompt) = content else {
            return;
        };

        let client = self.client.clone();
        let llm = Llm::new(llm_config.clone());
        let channel_id = message.channel_id;
        let reply_id = message.id;
        let prompt = prompt.to_string();

        tokio::spawn(async move {
            let flume = match client
                .flume(channel_id)
                .reply_id(reply_id)
                .components(Components {
                    inner: vec![Component {
                        id: Some(ComponentId(0)),
                        ty: ComponentType::Container {
                            components: vec![Component {
                                id: Some(ComponentId(1)),
                                ty: ComponentType::Text {
                                    content: "*loading...*".to_string(),
                                },
                                allow: None,
                            }],
                            color: "#33454f".parse().ok(),
                        },
                        allow: None,
                    }],
                })
                .await
            {
                Ok(flume) => flume,
                Err(e) => {
                    eprintln!("Failed to create flume: {:?}", e);
                    return;
                }
            };

            flume
                .update(FlumeDeltaCreate {
                    init: None,
                    append: vec![],
                    replace: vec![FlumeReplaceCreate {
                        target: ComponentId(0),
                        components: vec![ComponentCreate {
                            id: Some(ComponentId(0)),
                            ty: ComponentType::Container {
                                components: vec![Component {
                                    id: Some(ComponentId(1)),
                                    ty: ComponentType::Text {
                                        content: "*loading...*".to_string(),
                                    },
                                    allow: None,
                                }],
                                color: "#517082".parse().ok(),
                            },
                            allow: None,
                        }],
                    }],
                    delete: vec![],
                })
                .await;

            match llm.complete(prompt).await {
                Ok(stream) => {
                    futures::pin_mut!(stream);
                    let mut buffer = String::new();
                    let mut first = true;

                    while let Some(chunk) = stream.next().await {
                        let Some(content) = chunk.text() else {
                            continue;
                        };
                        buffer.push_str(content);

                        if first {
                            first = false;
                            flume
                                .update(FlumeDeltaCreate {
                                    init: None,
                                    append: vec![],
                                    replace: vec![FlumeReplaceCreate {
                                        target: ComponentId(1),
                                        components: vec![ComponentCreate {
                                            id: Some(ComponentId(1)),
                                            ty: ComponentType::Text {
                                                content: buffer.clone(),
                                            },
                                            allow: None,
                                        }],
                                    }],
                                    delete: vec![],
                                })
                                .await;
                        } else {
                            flume
                                .update(FlumeDeltaCreate {
                                    init: None,
                                    append: vec![FlumeAppendCreate {
                                        target: ComponentId(1),
                                        components: vec![ComponentCreate {
                                            id: Some(ComponentId(1)),
                                            ty: ComponentType::Text {
                                                content: content.to_string(),
                                            },
                                            allow: None,
                                        }],
                                    }],
                                    replace: vec![],
                                    delete: vec![],
                                })
                                .await;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error calling LLM: {:?}", e);
                }
            }

            flume.commit().await;
        });
    }
}
