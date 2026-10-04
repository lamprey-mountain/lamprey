use std::{
    collections::HashMap,
    mem,
    ops::RangeFrom,
    sync::{Arc, Mutex},
    time::Instant,
};

use lamprey_common::v1::types::misc::metadata::Metadata;
use lamprey_common::v1::types::unfurl::log::{self, EntryKind};
use tracing::{Event, Subscriber, field::Visit, span};
use tracing_subscriber::{Layer, layer::Context, registry::LookupSpan};

/// a tracing layer that collects diagnostics into a Vec of [`log::Entry`].
pub struct DebugLayer {
    start: Instant,
    inner: Arc<Mutex<Inner>>, // PERF: surely i dont need to use a Mutex here, right?
}

/// utility to collect metaata from tracing attributes
struct FieldVisitor {
    metadata: Metadata,
}

impl Visit for FieldVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.metadata
            .insert(field.name().to_owned(), format!("{:?}", value));
    }
}

struct Inner {
    ids: RangeFrom<u64>,
    nodes: HashMap<u64, Node>,
    roots: Vec<u64>,
}

struct Node {
    entry: log::Entry,
    children: Vec<u64>,
}

/// newtype for the extension map
struct EntryId(u64);

impl Default for Inner {
    fn default() -> Self {
        Self {
            ids: 1..,
            nodes: Default::default(),
            roots: Default::default(),
        }
    }
}

impl Node {
    fn new(entry: log::Entry) -> Self {
        Self {
            entry,
            children: vec![],
        }
    }
}

impl DebugLayer {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            inner: Default::default(),
        }
    }

    fn insert(&self, parent: Option<u64>, level: log::Level, kind: EntryKind) -> u64 {
        let now = self.start.elapsed().as_millis() as u64;
        let mut inner = self.inner.lock().unwrap();
        let id = inner.ids.next().unwrap();

        // ended and children will be filled in later
        let entry = log::Entry {
            id,
            started: now,
            ended: now,
            level,
            children: vec![],
            kind,
        };

        inner.nodes.insert(id, Node::new(entry));
        match parent.and_then(|p| inner.nodes.get_mut(&p)) {
            Some(p) => p.children.push(id),
            None => inner.roots.push(id),
        }
        id
    }

    pub fn take_entries(&self) -> Vec<log::Entry> {
        let mut inner = self.inner.lock().unwrap();
        let roots = mem::take(&mut inner.roots);

        fn build(n: &mut HashMap<u64, Node>, id: u64) -> Option<log::Entry> {
            let node = n.remove(&id)?;
            let mut e = node.entry;
            e.children = node
                .children
                .into_iter()
                .filter_map(|c| build(n, c))
                .collect();
            Some(e)
        }

        roots
            .into_iter()
            .filter_map(|r| build(&mut inner.nodes, r))
            .collect()
    }
}

impl<S> Layer<S> for DebugLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).unwrap();
        let parent = span
            .parent()
            .and_then(|p| p.extensions().get::<EntryId>().map(|e| e.0));

        let meta = attrs.metadata();

        let mut visitor = FieldVisitor {
            metadata: Metadata::default(),
        };
        attrs.record(&mut visitor);

        // TODO: handle `EntryKind::Http`
        let kind = EntryKind::Other {
            message: meta.name().to_owned(),
            attributes: visitor.metadata,
        };
        let level = convert_level(meta.level());
        let entry_id = self.insert(parent, level, kind);
        span.extensions_mut().insert(EntryId(entry_id));
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let parent = ctx
            .event_span(event)
            .and_then(|s| s.extensions().get::<EntryId>().map(|e| e.0));

        let mut visitor = FieldVisitor {
            metadata: Metadata::default(),
        };
        event.record(&mut visitor);

        let message = visitor
            .metadata
            .remove("message")
            .unwrap_or_else(|| event.metadata().name().to_owned());

        let kind = EntryKind::Other {
            message,
            attributes: visitor.metadata,
        };
        let level = convert_level(event.metadata().level());
        self.insert(parent, level, kind);
    }

    fn on_close(&self, id: span::Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(&id) else { return };

        let Some(eid) = span.extensions().get::<EntryId>().map(|e| e.0) else {
            return;
        };

        let mut inner = self.inner.lock().unwrap();
        let now = self.start.elapsed().as_millis() as u64;
        if let Some(n) = inner.nodes.get_mut(&eid) {
            n.entry.ended = now;
        }
    }
}

/// convert a tracing log level into a lamprey log level
fn convert_level(l: &tracing::Level) -> log::Level {
    match *l {
        tracing::Level::TRACE => log::Level::Trace,
        tracing::Level::DEBUG => log::Level::Debug,
        tracing::Level::INFO => log::Level::Info,
        tracing::Level::WARN => log::Level::Warning,
        tracing::Level::ERROR => log::Level::Error,
    }
}
