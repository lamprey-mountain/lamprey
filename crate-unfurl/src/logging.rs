use lamprey_common::v1::types::unfurl::log;
use tracing::{Subscriber, span};

/// captures tracing logs for debugging
#[derive(Default)]
pub struct DebugSubscriber(Vec<log::Entry>);

impl DebugSubscriber {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn into_entries(self) -> Vec<log::Entry> {
        self.0
    }
}

impl Subscriber for DebugSubscriber {
    fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, span: &span::Attributes<'_>) -> span::Id {
        todo!()
    }

    fn record(&self, span: &span::Id, values: &span::Record<'_>) {
        todo!()
    }

    fn record_follows_from(&self, span: &span::Id, follows: &span::Id) {
        todo!()
    }

    fn event(&self, event: &tracing::Event<'_>) {
        todo!()
    }

    fn enter(&self, span: &span::Id) {
        todo!()
    }

    fn exit(&self, span: &span::Id) {
        todo!()
    }
}
