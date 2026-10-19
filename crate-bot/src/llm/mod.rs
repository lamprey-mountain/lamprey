//! contains [`Llm`], a llm client

use std::collections::VecDeque;
use std::sync::Arc;

use crate::config::ConfigLlm;
use futures::stream::{self, BoxStream};
use futures::{Stream, StreamExt, TryStreamExt};
use tokio_util::codec::FramedRead;
use tokio_util::codec::LinesCodec;
use tokio_util::io::StreamReader;

mod api;
pub mod tools;

// TEMP: reexport everything for compat
pub use api::*;
pub use tools::{Context, Tool, ToolDef};

/// how many times a completion may call tools before it is cut off
const MAX_TOOL_ROUNDS: usize = 8;

/// a chunk of a streamed [`Llm::complete`] call
#[derive(Debug)]
pub enum CompletionChunk {
    /// a piece of assistant text
    Text(String),

    /// the model asked to call a tool
    ToolCall(ToolCall),

    /// the result of a tool call, after it was executed
    ToolResult(ToolResult),
}

/// the result of executing a [`ToolCall`]
#[derive(Debug)]
pub struct ToolResult {
    /// the tool call this is a result for
    pub call: ToolCall,

    /// what the tool returned
    pub output: String,
}

/// a client for an openai compatible api
pub struct Llm {
    http: reqwest::Client,
    config: ConfigLlm,
    tools: Vec<Arc<dyn Tool>>,
}

/// state for a [`Llm::complete`] stream
struct Completion<'a> {
    llm: &'a Llm,
    ctx: Context,
    messages: Vec<ChatMessage>,
    inner: Option<BoxStream<'static, CompletionChunk>>,
    calls: Vec<ToolCall>,
    text: String,
    pending: VecDeque<CompletionChunk>,
    rounds: usize,
}

/// parses openai's streaming response format
#[derive(Default)]
struct Parser {
    tool_calls: Vec<PartialToolCall>,
}

#[derive(Default)]
struct PartialToolCall {
    id: String,
    name: String,
    arguments: String,
}

impl Llm {
    /// create a new client
    pub fn new(config: ConfigLlm) -> Self {
        Self {
            http: reqwest::Client::new(),
            config,
            tools: vec![Arc::new(tools::datetime())],
        }
    }

    /// the configuration this client was built with
    pub fn config(&self) -> &ConfigLlm {
        &self.config
    }

    /// add a tool the model may call
    pub fn add_tool(&mut self, tool: impl Tool + 'static) {
        self.tools.push(Arc::new(tool));
    }

    /// execute a tool call requested by the model
    pub fn call_tool(&self, ctx: Context, call: &ToolCall) -> String {
        let Some(tool) = self
            .tools
            .iter()
            .find(|tool| tool.name() == call.function.name)
        else {
            return format!("error: unknown tool `{}`", call.function.name);
        };

        let arguments =
            serde_json::from_str(&call.function.arguments).unwrap_or(serde_json::Value::Null);
        tool.call(ctx, arguments)
    }

    /// generate a new chat completion, returning a stream of chunks.
    ///
    /// if the model calls tools, they are executed and their results are fed
    /// back to the model until it produces a reply without further tool calls.
    pub async fn complete(
        &self,
        ctx: Context,
        messages: &[ChatMessage],
    ) -> reqwest::Result<impl Stream<Item = CompletionChunk> + '_> {
        let inner = self.complete_step(messages).await?;
        let state = Completion {
            llm: self,
            ctx,
            messages: messages.to_vec(),
            inner: Some(Box::pin(inner)),
            calls: Vec::new(),
            text: String::new(),
            pending: VecDeque::new(),
            rounds: 0,
        };

        Ok(stream::unfold(state, |mut state| async move {
            state.next().await.map(|chunk| (chunk, state))
        }))
    }

    /// ask the model for a short title describing a conversation
    pub async fn generate_title(&self, ctx: Context, messages: &[ChatMessage]) -> Option<String> {
        let mut title_messages = Vec::with_capacity(messages.len() + 1);
        title_messages.push(ChatMessage {
            role: Role::System,
            content: vec![ContentPart::Text {
                text: "Generate a short title (a few words) describing the following \
                       conversation. Respond with only the title, without quotes or \
                       trailing punctuation."
                    .to_string(),
            }],
            tool_calls: None,
            tool_call_id: None,
        });
        title_messages.extend_from_slice(messages);

        let stream = self.complete(ctx, &title_messages).await.ok()?;
        futures::pin_mut!(stream);

        let mut title = String::new();
        while let Some(chunk) = stream.next().await {
            if let CompletionChunk::Text(text) = chunk {
                title.push_str(&text);
            }
        }

        let title = title
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())?
            .trim_matches('"')
            .trim();
        (!title.is_empty()).then(|| title.chars().take(64).collect())
    }

    /// send a single streaming request to the api
    async fn complete_step(
        &self,
        messages: &[ChatMessage],
    ) -> reqwest::Result<impl Stream<Item = CompletionChunk> + use<>> {
        let request = CompletionRequest {
            model: self.config.model.clone(),
            messages: messages.to_vec(),
            stream: true,
            tools: self.tools.iter().map(|tool| tool.definition()).collect(),
            tool_choice: Some("auto"),
        };

        let response = self
            .http
            .post(format!("{}/chat/completions", self.config.base_url))
            .bearer_auth(&self.config.token)
            .json(&request)
            .send()
            .await?
            .error_for_status()?;

        let stream = response.bytes_stream().map_err(std::io::Error::other);
        let reader = StreamReader::new(stream);
        let lines = FramedRead::new(reader, LinesCodec::new())
            .filter_map(|line| futures::future::ready(line.ok()))
            .chain(stream::once(futures::future::ready(
                "data: [DONE]".to_string(),
            )));

        Ok(lines
            .scan(Parser::default(), |parser, line| {
                futures::future::ready(Some(parser.feed(&line)))
            })
            .flat_map(stream::iter))
    }
}

impl Completion<'_> {
    /// get the next parsed chunk of the response
    async fn next(&mut self) -> Option<CompletionChunk> {
        loop {
            if let Some(chunk) = self.pending.pop_front() {
                return Some(chunk);
            }

            match self.inner.as_mut()?.next().await {
                Some(CompletionChunk::Text(text)) => {
                    self.text.push_str(&text);
                    return Some(CompletionChunk::Text(text));
                }
                Some(CompletionChunk::ToolCall(call)) => {
                    self.calls.push(call.clone());
                    return Some(CompletionChunk::ToolCall(call));
                }
                // the low level request stream never yields tool results
                Some(CompletionChunk::ToolResult(_)) => unreachable!(),
                None => self.next_round().await?,
            }
        }
    }

    async fn next_round(&mut self) -> Option<()> {
        self.inner = None;

        let calls = std::mem::take(&mut self.calls);
        if calls.is_empty() {
            return None;
        }

        self.rounds += 1;
        if self.rounds > MAX_TOOL_ROUNDS {
            tracing::error!("llm exceeded {MAX_TOOL_ROUNDS} tool calls, stopping");
            return None;
        }

        // record assistant's message
        let content = if self.text.is_empty() {
            Vec::new()
        } else {
            vec![ContentPart::Text {
                text: std::mem::take(&mut self.text),
            }]
        };
        self.messages.push(ChatMessage {
            role: Role::Assistant,
            content,
            tool_calls: Some(calls.clone()),
            tool_call_id: None,
        });

        // run tools
        // PERF: run in parallel?
        for call in calls {
            let output = self.llm.call_tool(self.ctx.clone(), &call);
            self.messages.push(ChatMessage {
                role: Role::Tool,
                content: vec![ContentPart::Text {
                    text: output.clone(),
                }],
                tool_calls: None,
                tool_call_id: Some(call.id.clone()),
            });
            self.pending
                .push_back(CompletionChunk::ToolResult(ToolResult { call, output }));
        }

        // send tool results to the model
        match self.llm.complete_step(&self.messages).await {
            Ok(next) => {
                self.inner = Some(Box::pin(next));
                Some(())
            }
            Err(e) => {
                tracing::error!("failed to continue completion after tool call: {e}");
                None
            }
        }
    }
}

impl PartialToolCall {
    fn apply(&mut self, delta: &ToolCallDelta) {
        if let Some(id) = &delta.id {
            self.id.clone_from(id);
        }
        if let Some(function) = &delta.function {
            if let Some(name) = &function.name {
                self.name.clone_from(name);
            }
            if let Some(arguments) = &function.arguments {
                self.arguments.push_str(arguments);
            }
        }
    }
}

impl Parser {
    /// process a single sse line, returning any chunks to emit
    fn feed(&mut self, line: &str) -> Vec<CompletionChunk> {
        let Some(data) = line.strip_prefix("data: ") else {
            return Vec::new();
        };

        if data == "[DONE]" {
            return self.flush();
        }

        let Ok(chunk) = serde_json::from_str::<ChatCompletionChunk>(data) else {
            return Vec::new();
        };

        let mut chunks = Vec::new();
        if let Some(text) = chunk.text().filter(|text| !text.is_empty()) {
            chunks.push(CompletionChunk::Text(text.to_string()));
        }

        for delta in chunk.tool_calls() {
            if self.tool_calls.len() <= delta.index {
                self.tool_calls
                    .resize_with(delta.index + 1, PartialToolCall::default);
            }
            self.tool_calls[delta.index].apply(delta);
        }

        chunks
    }

    /// turn any accumulated tool call deltas into complete tool calls
    fn flush(&mut self) -> Vec<CompletionChunk> {
        std::mem::take(&mut self.tool_calls)
            .into_iter()
            .filter(|call| !call.name.is_empty())
            .map(|call| {
                CompletionChunk::ToolCall(ToolCall {
                    id: call.id,
                    type_: "function".to_string(),
                    function: FunctionCall {
                        name: call.name,
                        arguments: call.arguments,
                    },
                })
            })
            .collect()
    }
}
