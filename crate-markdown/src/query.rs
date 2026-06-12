use crate::ast::block::Header;
use crate::ast::inline::{CustomEmoji, Link, Mention};
use crate::prelude::*;
use crate::query::decorations::DecorationGenerator;

mod decorations;

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

// TODO: wasm support
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
        let gen = match span {
            Some(span) => DecorationGenerator::new_span(root, span),
            None => DecorationGenerator::new_full(root),
        };
        gen.generate().into_iter()
    }
}

impl<T: Queryable + ?Sized> QueryableExt for T {}
