use crate::ast::block::Header;
use crate::ast::inline::{CustomEmoji, Link, Mention};
use crate::prelude::*;
use crate::query::decorations::DecorationGenerator;

mod decorations;

#[cfg(feature = "wasm")]
mod wasm;

pub use decorations::{Decoration, DecorationKind};

pub trait Queryable {
    // TEMP: i probably want some kind of visitor pattern, or something that makes transforms easier
    fn get_root(&self) -> SyntaxNode;
}

impl Queryable for Ref<Tree> {
    fn get_root(&self) -> SyntaxNode {
        self.root()
    }
}

pub trait QueryableExt: Queryable {
    /// iterate over all links
    fn iter_links(&self) -> impl Iterator<Item = Link> {
        self.get_root().descendants().filter_map(Link::cast)
    }

    /// iterate over all mentions
    fn iter_mentions(&self) -> impl Iterator<Item = Mention> {
        self.get_root()
            .descendants_with_tokens()
            .filter_map(|element| element.into_token().and_then(Mention::cast))
    }

    /// iterate over all emoji
    // TODO: iterate over unicode emoji too
    fn iter_emoji(&self) -> impl Iterator<Item = CustomEmoji> {
        self.get_root()
            .descendants_with_tokens()
            .filter_map(|element| element.into_token().and_then(CustomEmoji::cast))
    }

    /// iterate over all headers
    fn iter_headers(&self) -> impl Iterator<Item = Header> {
        self.get_root().descendants().filter_map(Header::cast)
    }

    /// iterate over some decorations
    fn iter_decorations(&self, span: Option<Span>) -> impl Iterator<Item = Decoration> {
        let root = self.get_root();
        let decos = match span {
            Some(span) => DecorationGenerator::new_span(root, span),
            None => DecorationGenerator::new_full(root),
        };
        decos.generate().into_iter()
    }

    /// check if this document contains only emoji. if it does, returns the number of emoji contained within.
    fn only_emoji(&self) -> Option<u32> {
        let root = self.get_root();
        let mut emoji_count = 0;

        for el in root.descendants_with_tokens() {
            match el.kind() {
                NodeKind::Text(TextKind::CustomEmoji) | NodeKind::Text(TextKind::UnicodeEmoji) => {
                    emoji_count += 1;
                }

                // ignore whitespace
                NodeKind::Text(TextKind::Newline) => {}
                NodeKind::Text(TextKind::Text)
                    if el.as_token()?.text().chars().all(|c| c.is_whitespace()) => {}

                // some root level elements are ignored
                NodeKind::Document => {}
                NodeKind::Block(BlockKind::Paragraph) => {}

                // this isn't an emoji!
                _ => return None,
            }
        }

        Some(emoji_count)
    }
}

impl<T: Queryable + ?Sized> QueryableExt for T {}
