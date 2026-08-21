use crate::{bot::Bot, prelude::*};
use common::v1::types::Message;
use common::v1::types::MessageType;
use common::v1::types::components::{
    Component, ComponentCreate, ComponentId, ComponentType, Components,
};
use common::v1::types::flume::{FlumeAppendCreate, FlumeDeltaCreate, FlumeReplaceCreate};
use futures::StreamExt;
use futures::TryStreamExt;
use serde::Deserialize;
use tokio_util::codec::FramedRead;
use tokio_util::codec::LinesCodec;
use tokio_util::io::StreamReader;

#[derive(Deserialize, Debug)]
struct ChatCompletionChunk {
    choices: Vec<Choice>,
}

#[derive(Deserialize, Debug)]
struct Choice {
    delta: Delta,
}

#[derive(Deserialize, Debug)]
struct Delta {
    content: Option<String>,
}

impl Bot {
    pub(crate) fn handle_llm(&self, message: &Message) {
        let Some(llm) = &self.config.llm else {
            return;
        };

        let Some(current_user_id) = self.user_id else {
            return;
        };

        if message.author_id == current_user_id {
            return;
        }

        if !llm.channel_ids.contains(&message.channel_id) {
            return;
        }

        let content = match &message.latest_version.message_type {
            MessageType::DefaultMarkdown(m) => m.content.as_deref(),
            _ => None,
        };

        let Some(prompt) = content else {
            return;
        };

        let client = self.client.clone();
        let llm_config = llm.clone();
        let channel_id = message.channel_id;
        let reply_id = message.id;
        let prompt = prompt.to_string();

        // TODO: better error handling, don't panic
        tokio::spawn(async move {
            let flume = client
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
                            color: Some("#33454f".parse().unwrap()),
                        },
                        allow: None,
                    }],
                })
                .await
                .expect("Failed to create flume");

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
                                color: Some("#517082".parse().unwrap()),
                            },
                            allow: None,
                        }],
                    }],
                    delete: vec![],
                })
                .await;

            let http = reqwest::Client::new();
            let mut buffer = String::new();
            let mut first = true;

            let response = http
                .post(format!("{}/v1/chat/completions", llm_config.base_url))
                .bearer_auth(&llm_config.token)
                .json(&serde_json::json!({
                    "model": llm_config.model,
                    "messages": [{"role": "user", "content": prompt}],
                    "stream": true,
                }))
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let stream = resp
                        .bytes_stream()
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));
                    let reader = StreamReader::new(stream);
                    let mut lines = FramedRead::new(reader, LinesCodec::new());

                    while let Some(Ok(line)) = lines.next().await {
                        if line.starts_with("data: ") {
                            let data = &line["data: ".len()..];
                            if data == "[DONE]" {
                                break;
                            }
                            if let Ok(parsed) = serde_json::from_str::<ChatCompletionChunk>(data) {
                                if let Some(content) = parsed.choices[0].delta.content.as_ref() {
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
                                                        id: Some(ComponentId(1)), // This is probably wrong, should be new component
                                                        ty: ComponentType::Text {
                                                            content: content.clone(),
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
