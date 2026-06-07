use crate::prelude::*;
use crate::render::{Heading, Link, Mention};
use crate::tree::node::Node;

pub mod block;
pub mod inline;
pub mod transform;

/// abstract syntax tree
///
/// this is a strongly typed node that corresponds to an underlying parser node
pub trait Ast: Sized {
    /// attempt to convert a node into this
    fn cast(node: Node) -> Result<Self, Node>;

    /// get the underlying node
    fn node(&self) -> &Node;
}

// NOTE: do these need to be separate traits?
pub trait Render {
    /// render to html
    fn to_html(&self) -> String;

    /// render to plaintext, stripping any formatting
    fn to_plain(&self) -> String;
}

// pub trait AstExt {
//     fn strip_emoji(&mut self, allowed_emojis: Vec<Uuid>) -> StripEmoji;
// }

pub trait Queryable {
    /// iterate over all links
    fn iter_links(&self) -> impl Iterator<Item = Link>;

    /// iterate over all mentions
    fn iter_mentions(&self) -> impl Iterator<Item = Mention>;

    // TODO: iter_emoji

    /// iterate over all headings
    fn iter_headings(&self) -> impl Iterator<Item = Heading>;

    // /// iterate over all decorations
    // // NOTE: maybe i want a more efficient api? see parser EditResponse struct
    // // NOTE: i could also have `fn decorations(&self) -> &Decorations` to access resolved decos, unsure if js-wasm boundary overhead is too much though
    // fn iter_decorations(&self) -> impl Iterator<Item = Decoration>;
}
