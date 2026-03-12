//! Pull parser events for streaming markdown document processing.
//!
//! This module provides an iterator-based API for walking the AST,
//! enabling easy composition and transformation of renderers.
//!
//! # Example
//! ```
//! use lamprey_markdown::{Parser, Ast};
//! use lamprey_markdown::events::{Event, EventIterator, EventFilter};
//!
//! let parser = Parser::default();
//! let ast = Ast::new(parser.parse("**hello** world"));
//!
//! // Filter out emphasis
//! let events = ast.events()
//!     .filter_events(|e| !matches!(e, Event::Start(Tag::Emphasis) | Event::End(Tag::Emphasis)));
//!
//! // Transform text
//! let events = events.map(|e| match e {
//!     Event::Text(t) => Event::Text(t.replace("hello", "goodbye").into()),
//!     _ => e,
//! });
//!
//! // Collect to string
//! let output: String = events.map(|e| e.text()).collect();
//! ```

use crate::ast::{
    AngleBracketLink, Ast, AstNode, Autolink, BlockQuote, CodeBlock, Emoji, Emphasis, Header,
    InlineCode, Link, List, ListItem, Mention, Paragraph, Strikethrough, Strong,
};
use crate::parser::{SyntaxKind, SyntaxNode};
use rowan::{NodeOrToken, TextRange};
use std::borrow::Cow;

/// A tag representing the start or end of a block or inline element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tag<'a> {
    // Block elements
    Paragraph,
    Header(u8), // level (1-6)
    List(bool), // true = numbered, false = bullet
    ListItem,
    BlockQuote,
    CodeBlock,

    // Inline elements
    Strong,
    Emphasis,
    Strikethrough,
    InlineCode,
    Link {
        dest: Cow<'a, str>,
        title: Option<Cow<'a, str>>,
    },
    Autolink,
    AngleBracketLink,
    Mention,
    Emoji {
        animated: bool,
    },
}

/// A rendering event emitted by the pull parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event<'a> {
    /// Start of a block or inline element.
    Start(Tag<'a>),
    /// End of a block or inline element.
    End(Tag<'a>),
    /// Text content.
    Text(Cow<'a, str>),
    /// Code content (inside inline code or code blocks).
    Code(Cow<'a, str>),
    /// Soft line break (single newline within paragraph).
    SoftBreak,
    /// Hard line break (double newline or explicit break).
    HardBreak,
    /// Horizontal rule.
    Rule,
    /// HTML content (if supported).
    Html(Cow<'a, str>),
    /// Source range for the event (for source maps).
    SourceSpan(TextRange),
}

impl<'a> Event<'a> {
    /// Get the text content of an event, if any.
    pub fn text(&self) -> Cow<'a, str> {
        match self {
            Event::Text(t) => t.clone(),
            Event::Code(t) => t.clone(),
            Event::Html(t) => t.clone(),
            _ => Cow::Borrowed(""),
        }
    }

    /// Check if this is a start event.
    pub fn is_start(&self) -> bool {
        matches!(self, Event::Start(_))
    }

    /// Check if this is an end event.
    pub fn is_end(&self) -> bool {
        matches!(self, Event::End(_))
    }
}

/// An iterator that emits rendering events from an AST.
///
/// This is a pull parser - it generates events on-demand as you iterate,
/// enabling lazy evaluation and efficient composition.
pub struct EventIterator<'a> {
    /// Stack of nodes being traversed
    stack: Vec<IterState<'a>>,
    /// Source text for extracting content
    source: &'a str,
    /// Whether we've emitted the initial Document start
    started: bool,
    /// Pending events to emit before continuing traversal
    pending: Vec<Event<'a>>,
}

#[derive(Clone)]
enum IterState<'a> {
    Document {
        remaining: Vec<SyntaxNode>,
    },
    Paragraph {
        node: Paragraph,
        pos: usize,
        child_idx: usize,
    },
    Header {
        node: Header,
        level: u8,
        child_idx: usize,
    },
    List {
        node: List,
        child_idx: usize,
    },
    ListItem {
        node: ListItem,
        child_idx: usize,
    },
    BlockQuote {
        node: BlockQuote,
        child_idx: usize,
    },
    CodeBlock {
        node: CodeBlock,
        emitted: bool,
    },
    Strong {
        node: Strong,
        child_idx: usize,
    },
    Emphasis {
        node: Emphasis,
        child_idx: usize,
    },
    Strikethrough {
        node: Strikethrough,
        child_idx: usize,
    },
    InlineCode {
        node: InlineCode,
        emitted: bool,
    },
    Link {
        node: Link,
        child_idx: usize,
    },
    Autolink {
        node: Autolink,
        emitted: bool,
    },
    AngleBracketLink {
        node: AngleBracketLink,
        emitted: bool,
    },
    Mention {
        node: Mention,
        emitted: bool,
    },
    Emoji {
        node: Emoji,
        emitted: bool,
    },
    Text {
        text: Cow<'a, str>,
        emitted: bool,
    },
}

impl<'a> EventIterator<'a> {
    /// Create a new event iterator from an AST.
    pub fn new(ast: &'a Ast) -> Self {
        let mut stack = Vec::new();

        // Start with document children
        if let Some(doc) = ast.document() {
            let children: Vec<_> = doc.syntax_node().children().collect();
            stack.push(IterState::Document {
                remaining: children,
            });
        }

        Self {
            stack,
            source: ast.source(),
            started: false,
            pending: Vec::new(),
        }
    }

    /// Wrap the iterator with source span information.
    ///
    /// Returns an iterator of (Event, TextRange) pairs.
    pub fn with_source_spans(self) -> EventWithSpanIterator<'a> {
        EventWithSpanIterator::new(self)
    }

    /// Merge consecutive text events into single events.
    ///
    /// This is useful for renderers that want to process text in larger chunks.
    pub fn merge_text(self) -> MergeTextIterator<'a, Self> {
        MergeTextIterator::new(self)
    }

    fn next_event(&mut self) -> Option<Event<'a>> {
        // First, emit any pending events
        if let Some(event) = self.pending.pop() {
            return Some(event);
        }

        loop {
            // Get current state - clone what we need to avoid borrow conflicts
            let current_state = self.stack.last().cloned();

            match current_state {
                None => return None,
                Some(IterState::Document { .. }) => {
                    // Pop and process Document state
                    if let Some(IterState::Document { mut remaining }) = self.stack.pop() {
                        if let Some(child) = remaining.pop() {
                            // Push remaining back
                            self.stack.push(IterState::Document { remaining });
                            // Push child state
                            self.push_state_for_node(child);
                        }
                    }
                }

                Some(IterState::Paragraph {
                    node,
                    pos,
                    mut child_idx,
                }) => {
                    self.stack.pop();
                    if pos == 0 {
                        self.stack.push(IterState::Paragraph {
                            node,
                            pos: 1,
                            child_idx,
                        });
                        return Some(Event::Start(Tag::Paragraph));
                    }

                    let children: Vec<_> = node.syntax_node().children_with_tokens().collect();
                    if child_idx < children.len() {
                        let child = children[child_idx].clone();
                        child_idx += 1;
                        self.stack.push(IterState::Paragraph {
                            node,
                            pos,
                            child_idx,
                        });
                        self.push_state_for_node_or_text(child);
                    }
                }

                Some(IterState::Header {
                    node,
                    level,
                    mut child_idx,
                }) => {
                    self.stack.pop();
                    if child_idx == 0 {
                        self.stack.push(IterState::Header {
                            node,
                            level,
                            child_idx: 1,
                        });
                        return Some(Event::Start(Tag::Header(level)));
                    }

                    let children: Vec<_> = node
                        .syntax_node()
                        .children()
                        .filter(|n| n.kind() != SyntaxKind::HeaderMarker)
                        .collect();

                    if child_idx - 1 < children.len() {
                        let child = children[child_idx - 1].clone();
                        child_idx += 1;
                        self.stack.push(IterState::Header {
                            node,
                            level,
                            child_idx,
                        });
                        self.push_state_for_node(child);
                    } else {
                        return Some(Event::End(Tag::Header(level)));
                    }
                }

                Some(IterState::List {
                    node,
                    mut child_idx,
                }) => {
                    self.stack.pop();
                    if child_idx == 0 {
                        let is_numbered = node.is_numbered();
                        self.stack.push(IterState::List { node, child_idx: 1 });
                        return Some(Event::Start(Tag::List(is_numbered)));
                    }

                    let children: Vec<_> = node.syntax_node().children().collect();
                    if child_idx < children.len() {
                        let child = children[child_idx].clone();
                        child_idx += 1;
                        self.stack.push(IterState::List { node, child_idx });
                        if let Some(item) = ListItem::cast(child) {
                            self.stack.push(IterState::ListItem {
                                node: item,
                                child_idx: 0,
                            });
                        }
                    } else {
                        return Some(Event::End(Tag::List(node.is_numbered())));
                    }
                }

                Some(IterState::ListItem {
                    node,
                    mut child_idx,
                }) => {
                    self.stack.pop();
                    if child_idx == 0 {
                        self.stack.push(IterState::ListItem { node, child_idx: 1 });
                        return Some(Event::Start(Tag::ListItem));
                    }

                    let children: Vec<_> = node
                        .syntax_node()
                        .children_with_tokens()
                        .filter(|n| match n {
                            NodeOrToken::Node(n) => n.kind() != SyntaxKind::ListMarker,
                            NodeOrToken::Token(_) => true,
                        })
                        .collect();

                    if child_idx - 1 < children.len() {
                        let child = children[child_idx - 1].clone();
                        child_idx += 1;
                        self.stack.push(IterState::ListItem { node, child_idx });
                        self.push_state_for_node_or_text(child);
                    } else {
                        return Some(Event::End(Tag::ListItem));
                    }
                }

                Some(IterState::BlockQuote {
                    node,
                    mut child_idx,
                }) => {
                    self.stack.pop();
                    if child_idx == 0 {
                        self.stack
                            .push(IterState::BlockQuote { node, child_idx: 1 });
                        return Some(Event::Start(Tag::BlockQuote));
                    }

                    let children: Vec<_> = node
                        .syntax_node()
                        .children()
                        .filter(|n| n.kind() != SyntaxKind::BlockQuoteMarker)
                        .collect();

                    if child_idx - 1 < children.len() {
                        let child = children[child_idx - 1].clone();
                        child_idx += 1;
                        self.stack.push(IterState::BlockQuote { node, child_idx });
                        self.push_state_for_node(child);
                    } else {
                        return Some(Event::End(Tag::BlockQuote));
                    }
                }

                Some(IterState::CodeBlock { node, emitted }) => {
                    self.stack.pop();
                    if !emitted {
                        self.stack.push(IterState::CodeBlock {
                            node,
                            emitted: true,
                        });
                        return Some(Event::Start(Tag::CodeBlock));
                    }

                    let code = node.code();
                    if !code.is_empty() {
                        return Some(Event::Code(code.into()));
                    }

                    return Some(Event::End(Tag::CodeBlock));
                }

                Some(IterState::Strong {
                    node,
                    mut child_idx,
                }) => {
                    self.stack.pop();
                    if child_idx == 0 {
                        self.stack.push(IterState::Strong { node, child_idx: 1 });
                        return Some(Event::Start(Tag::Strong));
                    }

                    let children: Vec<_> = node
                        .syntax_node()
                        .children_with_tokens()
                        .filter(|n| match n {
                            NodeOrToken::Node(_) => true,
                            NodeOrToken::Token(t) => t.kind() != SyntaxKind::StrongDelimiter,
                        })
                        .collect();

                    if child_idx - 1 < children.len() {
                        let child = children[child_idx - 1].clone();
                        child_idx += 1;
                        self.stack.push(IterState::Strong { node, child_idx });
                        self.push_state_for_node_or_text(child);
                    } else {
                        return Some(Event::End(Tag::Strong));
                    }
                }

                Some(IterState::Emphasis {
                    node,
                    mut child_idx,
                }) => {
                    self.stack.pop();
                    if child_idx == 0 {
                        self.stack.push(IterState::Emphasis { node, child_idx: 1 });
                        return Some(Event::Start(Tag::Emphasis));
                    }

                    let children: Vec<_> = node
                        .syntax_node()
                        .children_with_tokens()
                        .filter(|n| match n {
                            NodeOrToken::Node(_) => true,
                            NodeOrToken::Token(t) => t.kind() != SyntaxKind::EmphasisDelimiter,
                        })
                        .collect();

                    if child_idx - 1 < children.len() {
                        let child = children[child_idx - 1].clone();
                        child_idx += 1;
                        self.stack.push(IterState::Emphasis { node, child_idx });
                        self.push_state_for_node_or_text(child);
                    } else {
                        return Some(Event::End(Tag::Emphasis));
                    }
                }

                Some(IterState::Strikethrough {
                    node,
                    mut child_idx,
                }) => {
                    self.stack.pop();
                    if child_idx == 0 {
                        self.stack
                            .push(IterState::Strikethrough { node, child_idx: 1 });
                        return Some(Event::Start(Tag::Strikethrough));
                    }

                    let children: Vec<_> = node
                        .syntax_node()
                        .children_with_tokens()
                        .filter(|n| match n {
                            NodeOrToken::Node(_) => true,
                            NodeOrToken::Token(t) => t.kind() != SyntaxKind::StrikethroughDelimiter,
                        })
                        .collect();

                    if child_idx - 1 < children.len() {
                        let child = children[child_idx - 1].clone();
                        child_idx += 1;
                        self.stack
                            .push(IterState::Strikethrough { node, child_idx });
                        self.push_state_for_node_or_text(child);
                    } else {
                        return Some(Event::End(Tag::Strikethrough));
                    }
                }

                Some(IterState::InlineCode { node, emitted }) => {
                    self.stack.pop();
                    if !emitted {
                        let code = node.code();
                        return Some(Event::Code(code.into()));
                    }
                    return Some(Event::Start(Tag::InlineCode));
                }

                Some(IterState::Link {
                    node,
                    mut child_idx,
                }) => {
                    self.stack.pop();
                    if child_idx == 0 {
                        let dest = node.destination();
                        self.stack.push(IterState::Link { node, child_idx: 1 });
                        return Some(Event::Start(Tag::Link {
                            dest: dest.into(),
                            title: None,
                        }));
                    }

                    let children: Vec<_> = node
                        .syntax_node()
                        .children_with_tokens()
                        .filter_map(|n| {
                            if let NodeOrToken::Node(n) = n {
                                if n.kind() == SyntaxKind::LinkText {
                                    return Some(n.children_with_tokens().collect::<Vec<_>>());
                                }
                            }
                            None
                        })
                        .flatten()
                        .filter(|n| match n {
                            NodeOrToken::Node(_) => true,
                            NodeOrToken::Token(t) => {
                                t.kind() != SyntaxKind::Text || (t.text() != "[" && t.text() != "]")
                            }
                        })
                        .collect();

                    if child_idx - 1 < children.len() {
                        let child = children[child_idx - 1].clone();
                        child_idx += 1;
                        self.stack.push(IterState::Link { node, child_idx });
                        self.push_state_for_node_or_text(child);
                    } else {
                        return Some(Event::End(Tag::Link {
                            dest: node.destination().into(),
                            title: None,
                        }));
                    }
                }

                Some(IterState::Autolink { node, emitted }) => {
                    self.stack.pop();
                    if !emitted {
                        self.stack.push(IterState::Autolink {
                            node,
                            emitted: true,
                        });
                        return Some(Event::Start(Tag::Autolink));
                    }
                    let url = node.url();
                    return Some(Event::Text(url.into()));
                }

                Some(IterState::AngleBracketLink { node, emitted }) => {
                    self.stack.pop();
                    if !emitted {
                        self.stack.push(IterState::AngleBracketLink {
                            node,
                            emitted: true,
                        });
                        return Some(Event::Start(Tag::AngleBracketLink));
                    }
                    let url = node.url();
                    return Some(Event::Text(url.into()));
                }

                Some(IterState::Mention { node, emitted }) => {
                    self.stack.pop();
                    if !emitted {
                        self.stack.push(IterState::Mention {
                            node,
                            emitted: true,
                        });
                        return Some(Event::Start(Tag::Mention));
                    }
                    let uuid = node.uuid();
                    return Some(Event::Text(uuid.into()));
                }

                Some(IterState::Emoji { node, emitted }) => {
                    self.stack.pop();
                    if !emitted {
                        let name = node.name();
                        let animated = name.starts_with('a');
                        self.stack.push(IterState::Emoji {
                            node,
                            emitted: true,
                        });
                        return Some(Event::Start(Tag::Emoji { animated }));
                    }
                    let name = node.name();
                    return Some(Event::Text(format!(":{}:", name).into()));
                }

                Some(IterState::Text { text, emitted }) => {
                    self.stack.pop();
                    if !emitted {
                        return Some(Event::Text(text));
                    }
                }
            }
        }
    }

    /// Push state for a node, or emit text for tokens
    fn push_state_for_node_or_text(
        &mut self,
        child: NodeOrToken<SyntaxNode, rowan::SyntaxToken<crate::parser::MyLang>>,
    ) {
        match child {
            NodeOrToken::Node(node) => {
                self.push_state_for_node(node);
            }
            NodeOrToken::Token(token) => {
                let text = token.text();
                match token.kind() {
                    SyntaxKind::Text => {
                        // Check for newlines
                        if text.contains('\n') {
                            let parts: Vec<String> =
                                text.split('\n').map(|s| s.to_string()).collect();
                            for (i, part) in parts.iter().enumerate() {
                                if !part.is_empty() {
                                    self.pending.push(Event::Text(Cow::Owned(part.clone())));
                                }
                                if i < parts.len() - 1 {
                                    self.pending.push(Event::SoftBreak);
                                }
                            }
                        } else if text.trim().is_empty() {
                            self.pending.push(Event::Text(Cow::Borrowed(" ")));
                        } else {
                            self.pending.push(Event::Text(Cow::Owned(text.to_string())));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn push_state_for_node(&mut self, node: SyntaxNode) {
        match node.kind() {
            SyntaxKind::Paragraph => {
                if let Some(para) = Paragraph::cast(node) {
                    self.stack.push(IterState::Paragraph {
                        node: para,
                        pos: 0,
                        child_idx: 0,
                    });
                }
            }
            SyntaxKind::Header => {
                if let Some(header) = Header::cast(node) {
                    let level = header.level();
                    self.stack.push(IterState::Header {
                        node: header,
                        level,
                        child_idx: 0,
                    });
                }
            }
            SyntaxKind::List => {
                if let Some(list) = List::cast(node) {
                    self.stack.push(IterState::List {
                        node: list,
                        child_idx: 0,
                    });
                }
            }
            SyntaxKind::BlockQuote => {
                if let Some(bq) = BlockQuote::cast(node) {
                    self.stack.push(IterState::BlockQuote {
                        node: bq,
                        child_idx: 0,
                    });
                }
            }
            SyntaxKind::CodeBlock => {
                if let Some(cb) = CodeBlock::cast(node) {
                    self.stack.push(IterState::CodeBlock {
                        node: cb,
                        emitted: false,
                    });
                }
            }
            SyntaxKind::Strong => {
                if let Some(strong) = Strong::cast(node) {
                    self.stack.push(IterState::Strong {
                        node: strong,
                        child_idx: 0,
                    });
                }
            }
            SyntaxKind::Emphasis => {
                if let Some(emph) = Emphasis::cast(node) {
                    self.stack.push(IterState::Emphasis {
                        node: emph,
                        child_idx: 0,
                    });
                }
            }
            SyntaxKind::Strikethrough => {
                if let Some(strike) = Strikethrough::cast(node) {
                    self.stack.push(IterState::Strikethrough {
                        node: strike,
                        child_idx: 0,
                    });
                }
            }
            SyntaxKind::InlineCode => {
                if let Some(code) = InlineCode::cast(node) {
                    self.stack.push(IterState::InlineCode {
                        node: code,
                        emitted: false,
                    });
                }
            }
            SyntaxKind::Link => {
                if let Some(link) = Link::cast(node) {
                    self.stack.push(IterState::Link {
                        node: link,
                        child_idx: 0,
                    });
                }
            }
            SyntaxKind::Autolink => {
                if let Some(link) = Autolink::cast(node) {
                    self.stack.push(IterState::Autolink {
                        node: link,
                        emitted: false,
                    });
                }
            }
            SyntaxKind::AngleBracketLink => {
                if let Some(link) = AngleBracketLink::cast(node) {
                    self.stack.push(IterState::AngleBracketLink {
                        node: link,
                        emitted: false,
                    });
                }
            }
            SyntaxKind::Mention => {
                if let Some(mention) = Mention::cast(node) {
                    self.stack.push(IterState::Mention {
                        node: mention,
                        emitted: false,
                    });
                }
            }
            SyntaxKind::Emoji => {
                if let Some(emoji) = Emoji::cast(node) {
                    self.stack.push(IterState::Emoji {
                        node: emoji,
                        emitted: false,
                    });
                }
            }
            _ => {}
        }
    }
}

impl<'a> Iterator for EventIterator<'a> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_event()
    }
}

/// Iterator adapter that merges consecutive text events.
pub struct MergeTextIterator<'a, I> {
    inner: I,
    pending_text: Option<String>,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a, I: Iterator<Item = Event<'a>>> MergeTextIterator<'a, I> {
    pub fn new(inner: I) -> Self {
        Self {
            inner,
            pending_text: None,
            _marker: std::marker::PhantomData,
        }
    }

    fn flush_pending(&mut self) -> Option<Event<'a>> {
        self.pending_text.take().map(|t| Event::Text(t.into()))
    }
}

impl<'a, I: Iterator<Item = Event<'a>>> Iterator for MergeTextIterator<'a, I> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.inner.next() {
                Some(Event::Text(t)) => match &mut self.pending_text {
                    Some(acc) => acc.push_str(&t),
                    None => self.pending_text = Some(t.to_string()),
                },
                Some(event) => {
                    if let Some(text) = self.flush_pending() {
                        self.pending_text = None;
                        return Some(text);
                    }
                    return Some(event);
                }
                None => return self.flush_pending(),
            }
        }
    }
}

/// Iterator adapter that yields (Event, TextRange) pairs.
pub struct EventWithSpanIterator<'a> {
    inner: EventIterator<'a>,
    current_span: Option<TextRange>,
}

impl<'a> EventWithSpanIterator<'a> {
    pub fn new(inner: EventIterator<'a>) -> Self {
        Self {
            inner,
            current_span: None,
        }
    }
}

impl<'a> Iterator for EventWithSpanIterator<'a> {
    type Item = (Event<'a>, Option<TextRange>);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|event| (event, None))
    }
}

/// Extension trait for event iterators with common transformations.
pub trait EventFilter<'a>: Iterator<Item = Event<'a>> + Sized {
    /// Filter events based on a predicate.
    fn filter_events<F>(self, f: F) -> std::iter::Filter<Self, F>
    where
        F: FnMut(&Event<'a>) -> bool,
    {
        self.filter(f)
    }

    /// Transform text events.
    fn map_text<F>(self, mut f: F) -> std::iter::Map<Self, impl FnMut(Event<'a>) -> Event<'a>>
    where
        F: FnMut(&str) -> String + 'static,
    {
        self.map(move |event| match event {
            Event::Text(t) => Event::Text(f(&t).into()),
            _ => event,
        })
    }

    /// Strip emphasis events (but keep content).
    fn strip_emphasis(self) -> std::iter::Filter<Self, fn(&Event<'a>) -> bool> {
        self.filter(|e| !matches!(e, Event::Start(Tag::Emphasis) | Event::End(Tag::Emphasis)))
    }

    /// Strip strong events (but keep content).
    fn strip_strong(self) -> std::iter::Filter<Self, fn(&Event<'a>) -> bool> {
        self.filter(|e| !matches!(e, Event::Start(Tag::Strong) | Event::End(Tag::Strong)))
    }

    /// Strip strikethrough events (but keep content).
    fn strip_strikethrough(self) -> std::iter::Filter<Self, fn(&Event<'a>) -> bool> {
        self.filter(|e| {
            !matches!(
                e,
                Event::Start(Tag::Strikethrough) | Event::End(Tag::Strikethrough)
            )
        })
    }

    /// Strip emoji events.
    fn strip_emoji(self) -> std::iter::Filter<Self, fn(&Event<'a>) -> bool> {
        self.filter(|e| {
            !matches!(
                e,
                Event::Start(Tag::Emoji { .. }) | Event::End(Tag::Emoji { .. })
            )
        })
    }
}

impl<'a, I: Iterator<Item = Event<'a>>> EventFilter<'a> for I {}

/// Add events() method to Ast
impl Ast {
    /// Get an iterator over rendering events for this document.
    ///
    /// This is a pull parser - events are generated lazily as you iterate.
    ///
    /// # Example
    /// ```
    /// use lamprey_markdown::{Parser, Ast};
    /// use lamprey_markdown::events::Event;
    ///
    /// let parser = Parser::default();
    /// let ast = Ast::new(parser.parse("**hello** world"));
    ///
    /// for event in ast.events() {
    ///     match event {
    ///         Event::Start(tag) => println!("Start: {:?}", tag),
    ///         Event::Text(t) => println!("Text: {}", t),
    ///         Event::End(tag) => println!("End: {:?}", tag),
    ///         _ => {}
    ///     }
    /// }
    /// ```
    pub fn events(&self) -> EventIterator<'_> {
        EventIterator::new(self)
    }

    /// Get an iterator over rendering events with source spans.
    pub fn events_with_spans(&self) -> EventWithSpanIterator<'_> {
        EventWithSpanIterator::new(EventIterator::new(self))
    }
}
