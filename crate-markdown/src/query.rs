use crate::ast::inline::CustomEmojiData;
use crate::prelude::*;
use crate::render::{Heading, Link, Mention};

pub trait Queryable {
    // TEMP: i probably want some kind of visitor pattern, or something that makes transforms easier
    fn get_root(&self) -> SyntaxNode;
}

impl Queryable for Ref<Tree> {
    fn get_root(&self) -> SyntaxNode {
        self.root()
    }
}

// struct Queryable0;

trait Queryable0 {
    /// iterate over all links
    fn iter_links(&self) -> impl Iterator<Item = Link>;

    /// iterate over all mentions
    fn iter_mentions(&self) -> impl Iterator<Item = Mention>;

    /// iterate over all emoji
    fn iter_emoji(&self) -> impl Iterator<Item = CustomEmojiData>;

    /// iterate over all headings
    fn iter_headings(&self) -> impl Iterator<Item = Heading>;

    // /// iterate over all decorations
    // // NOTE: maybe i want a more efficient api? see parser EditResponse struct
    // // NOTE: i could also have `fn decorations(&self) -> &Decorations` to access resolved decos, unsure if js-wasm boundary overhead is too much though
    // fn iter_decorations(&self) -> impl Iterator<Item = Decoration>;
}
