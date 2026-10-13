use std::ops::ControlFlow;

use crate::{
    Plugin, Unfurler,
    plugin::{Head, HtmlSink, PluginHtml, UnfurlResult},
    unfurler::UnfurlerBuilder,
};
use async_trait::async_trait;
use html5ever::{
    local_name,
    tokenizer::{TagKind, Token},
};
use tracing::debug;

/// a plugin which parses json ld from html
#[derive(Debug, Default)]
pub struct JsonLdPlugin {
    _nope: (), // prevent users from manually constructing
}

impl Plugin for JsonLdPlugin {
    fn register(self, builder: UnfurlerBuilder) -> UnfurlerBuilder {
        builder.add_plugin_html(self)
    }
}

#[async_trait]
impl PluginHtml for JsonLdPlugin {
    async fn accept(&self, _unfurler: &Unfurler, _head: &Head<'_>) -> Option<Box<dyn HtmlSink>> {
        Some(Box::new(JsonLdSink::default()))
    }
}

#[derive(Default)]
struct JsonLdSink {
    in_script: bool,
    is_json_ld: bool,
    buffer: String,
}

#[async_trait]
impl HtmlSink for JsonLdSink {
    fn push(&mut self, token: &Token, _line_number: u64) -> ControlFlow<UnfurlResult> {
        match token {
            Token::TagToken(tag) => {
                if tag.kind == TagKind::StartTag && tag.name == local_name!("script") {
                    self.in_script = true;
                    self.is_json_ld = tag.attrs.iter().any(|attr| {
                        attr.name.local == local_name!("type")
                            && attr.value.as_ref() == "application/ld+json"
                    });
                } else if tag.kind == TagKind::EndTag && tag.name == local_name!("script") {
                    if self.is_json_ld {
                        // TODO: parse the collected buffer as JSON-LD
                        // For now, just logging for validation
                        debug!(json_ld = %self.buffer, "Found JSON-LD");
                    }
                    self.in_script = false;
                    self.is_json_ld = false;
                    self.buffer.clear();
                }
            }
            Token::CharacterTokens(chars) => {
                if self.in_script && self.is_json_ld {
                    self.buffer.push_str(chars);
                }
            }
            _ => {}
        }

        ControlFlow::Continue(())
    }

    async fn finish(self: Box<Self>) -> UnfurlResult {
        // TODO: parse json ld
        UnfurlResult::skip()
    }
}
