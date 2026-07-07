use crate::prelude::*;

/// a decoration that can be applied to the markdown source
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Decoration {
    pub span: Span,
    pub kind: DecorationKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum DecorationKind {
    Syntax,
    CodeLanguage,
    Emphasis,
    Strong,
    Spoiler,
    Link,
    Strikethrough,
    Code,
}

pub struct DecorationGenerator {
    root: SyntaxNode,
}

impl DecorationGenerator {
    pub fn new_span(root: SyntaxNode, span: Span) -> Self {
        let node = root.covering_element(span.into());
        let root = node.parent().unwrap_or(root);
        Self { root }
    }

    pub fn new_full(root: SyntaxNode) -> Self {
        Self { root }
    }

    pub fn generate(self) -> Vec<Decoration> {
        let mut decos = vec![];
        let root = self.root.clone();
        let el = SyntaxElement::Node(root);
        self.collect_decorations(&el, &mut decos);
        decos
    }
}

impl DecorationGenerator {
    fn collect_decorations(&self, el: &SyntaxElement, buffer: &mut Vec<Decoration>) {
        let kind = node_to_deco_kind(el.kind());

        if let Some(kind) = kind {
            buffer.push(Decoration {
                span: el.text_range().into(),
                kind,
            });
        }

        if let Some(node) = el.as_node() {
            for child in node.children_with_tokens() {
                self.collect_decorations(&child, buffer);
            }
        }
    }
}

fn node_to_deco_kind(kind: NodeKind) -> Option<DecorationKind> {
    match kind {
        // only decorate the text inside the node
        // NodeKind::Inline(InlineKind::Autolink) => Some(DecorationKind::Link),
        // NodeKind::Inline(InlineKind::Link) => Some(DecorationKind::Link),
        NodeKind::Inline(InlineKind::Code) => Some(DecorationKind::Code),
        NodeKind::Inline(InlineKind::Emphasis) => Some(DecorationKind::Emphasis),
        NodeKind::Inline(InlineKind::Spoiler) => Some(DecorationKind::Spoiler),
        NodeKind::Inline(InlineKind::Strikethrough) => Some(DecorationKind::Strikethrough),
        NodeKind::Inline(InlineKind::Strong) => Some(DecorationKind::Strong),
        NodeKind::Text(TextKind::CodeblockLang) => Some(DecorationKind::CodeLanguage),
        NodeKind::Text(TextKind::LinkUrl) => Some(DecorationKind::Link),
        NodeKind::Text(TextKind::Syntax) => Some(DecorationKind::Syntax),
        NodeKind::Text(TextKind::HeaderHashes) => Some(DecorationKind::Syntax),
        NodeKind::Text(TextKind::TableAlignment) => Some(DecorationKind::Syntax),
        NodeKind::Text(TextKind::ListPrefix) => Some(DecorationKind::Syntax),
        // custom elements?
        NodeKind::Text(TextKind::UnicodeEmoji) => todo!(),
        NodeKind::Text(TextKind::CustomEmoji) => todo!(),
        NodeKind::Text(TextKind::Mention) => todo!(),
        _ => None,
    }
}

#[cfg(any())]
#[cfg(feature = "serde")]
mod _s {
    use serde::Serialize;

    use crate::{
        prelude::Len,
        render::{Decoration, DecorationKind},
    };

    #[derive(Serialize)]
    struct Deco {
        span_start: Len,
        span_end: Len,
        kind: DecorationKind,
    }

    impl Serialize for Decoration {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            todo!()
        }
    }
}
