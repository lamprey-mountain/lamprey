//! types for the openai-compatible chat completions api

use serde::{Deserialize, Serialize};

use super::tools::ToolDef;

#[derive(Debug, Serialize)]
pub struct CompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolDef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<&'static str>,
}

#[derive(Debug, Deserialize)]
pub struct ChatCompletionChunk {
    pub choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub delta: Delta,
    // pub role: Role,
}

/// a streamed increment to a tool call
#[derive(Debug, Deserialize)]
pub struct ToolCallDelta {
    /// index of the tool call this delta belongs to
    pub index: usize,
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub function: Option<FunctionCallDelta>,
}

#[derive(Debug, Deserialize)]
pub struct FunctionCallDelta {
    pub name: Option<String>,
    pub arguments: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub function: FunctionCall,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionCall {
    pub name: String,

    /// the input for this tool as serialized json
    pub arguments: String,
}

#[derive(Debug, Deserialize)]
pub struct Delta {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCallDelta>>,
}

/// a single message in an llm conversation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<ContentPart>,

    /// tool calls requested by an assistant message
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,

    /// id of the tool call a tool result message is responding to
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentPart {
    #[serde(rename = "text")]
    Text { text: String },

    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrl },
    // add these later
    // #[serde(rename = "input_audio")]
    // InputAudio { input_audio: InputAudio },
    // #[serde(rename = "input_video")]
    // InputVideo { input_video: InputVideo },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
}

/// the role of a [`ChatMessage`]
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Developer,
    Tool,
}

impl ChatCompletionChunk {
    /// the first content delta of this chunk, if any
    pub fn text(&self) -> Option<&str> {
        self.choices.first()?.delta.content.as_deref()
    }

    /// the tool call deltas of this chunk, if any
    pub fn tool_calls(&self) -> impl Iterator<Item = &ToolCallDelta> {
        self.choices
            .iter()
            .filter_map(|choice| choice.delta.tool_calls.as_deref())
            .flatten()
    }
}
