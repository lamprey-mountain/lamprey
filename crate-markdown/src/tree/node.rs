// TODO: move to tree/mod.rs

use crate::prelude::*;

// NOTE: why does rowan need Ord?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MarkdownLanguage;

impl rowan::Language for MarkdownLanguage {
    type Kind = NodeKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        raw.into()
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        kind.into()
    }
}

pub type SyntaxNode = rowan::SyntaxNode<MarkdownLanguage>;
pub type SyntaxToken = rowan::SyntaxToken<MarkdownLanguage>;
pub type SyntaxElement = rowan::NodeOrToken<SyntaxNode, SyntaxToken>;

pub type GreenNode = rowan::GreenNode;
pub type GreenToken = rowan::GreenToken;
pub type GreenElement = rowan::NodeOrToken<GreenNode, GreenToken>;

// ------

// impl SyntaxData {
//     pub fn kind(&self) -> NodeKind {
//         self.kind
//     }

//     pub fn span(&self) -> Span {
//         self.span
//     }

//     pub(crate) fn offset_span(&mut self, delta: isize) {
//         if delta > 0 {
//             self.span.start += delta as Len;
//             self.span.end += delta as Len;
//         } else if delta < 0 {
//             // TODO: better error handling for this?
//             let abs_delta = (-delta) as Len;
//             self.span.start = self.span.start.saturating_sub(abs_delta);
//             self.span.end = self.span.end.saturating_sub(abs_delta);
//         }
//     }
// }

// impl SyntaxNode {
//     /// get the text of this node
//     pub fn text(&self) -> &str {
//         let span = self.node.span();
//         &self.tree.source()[span.start as usize..span.end as usize]
//     }

//     /// get the children of this node
//     pub fn children(&self) -> impl Iterator<Item = SyntaxNode> + '_ {
//         self.node.children.iter().map(|n| SyntaxNode {
//             tree: Ref::clone(&self.tree),
//             node: self.tree[*n].clone(),
//         })
//     }

//     // pub fn data -> &SyntaxData
// }
