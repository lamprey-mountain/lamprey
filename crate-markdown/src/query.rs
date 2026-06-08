use crate::ast::block::Header;
use crate::ast::inline::{CustomEmoji, Link, Mention};
use crate::prelude::*;

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
    fn iter_emoji(&self) -> impl Iterator<Item = CustomEmoji> {
        self.get_root()
            .descendants_with_tokens()
            .filter_map(|element| element.into_token().and_then(CustomEmoji::cast))
    }

    /// iterate over all headers
    fn iter_headers(&self) -> impl Iterator<Item = Header> {
        self.get_root().descendants().filter_map(Header::cast)
    }

    // /// iterate over all decorations
    // // NOTE: maybe i want a more efficient api? see parser EditResponse struct
    // // NOTE: i could also have `fn decorations(&self) -> &Decorations` to access resolved decos, unsure if js-wasm boundary overhead is too much though
    // fn iter_decorations(&self) -> impl Iterator<Item = Decoration>;
}

impl<T: Queryable + ?Sized> QueryableExt for T {}
