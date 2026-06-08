use crate::prelude::*;

// NOTE: is u32 correct here?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntaxIndex(pub(crate) u32);

// NOTE: how do i want to handle visibility?
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxData {
    pub(crate) kind: NodeKind,

    // NOTE: maybe i don't want to store Span so that identical nodes can be reused
    pub(crate) span: Span,

    pub(crate) children: Vec<SyntaxIndex>,
    // NOTE: rowan has these fields:
    // rc: Cell<u32>,
    // parent: Cell<Option<ptr::NonNull<NodeData>>>,
    // index: Cell<u32>,
    // green: Green,

    // /// Invariant: never changes after NodeData is created.
    // mutable: bool,
    // /// Absolute offset for immutable nodes, unused for mutable nodes.
    // offset: TextSize,
    // // The following links only have meaning when `mutable` is true.
    // first: Cell<*const NodeData>,
    // /// Invariant: never null if mutable.
    // next: Cell<*const NodeData>,
    // /// Invariant: never null if mutable.
    // prev: Cell<*const NodeData>,
}

// NOTE: how do i want to handle visibility?
pub struct SyntaxNode {
    pub(crate) tree: Ref<Tree>,
    pub(crate) node: SyntaxData,
}

impl SyntaxData {
    pub fn kind(&self) -> NodeKind {
        self.kind
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub(crate) fn offset_span(&mut self, delta: isize) {
        if delta > 0 {
            self.span.start += delta as Len;
            self.span.end += delta as Len;
        } else if delta < 0 {
            // TODO: better error handling for this?
            let abs_delta = (-delta) as Len;
            self.span.start = self.span.start.saturating_sub(abs_delta);
            self.span.end = self.span.end.saturating_sub(abs_delta);
        }
    }
}

impl SyntaxNode {
    /// get the text of this node
    pub fn text(&self) -> &str {
        let span = self.node.span();
        &self.tree.source()[span.start as usize..span.end as usize]
    }

    /// get the children of this node
    pub fn children(&self) -> impl Iterator<Item = SyntaxNode> + '_ {
        self.node.children.iter().map(|n| SyntaxNode {
            tree: Ref::clone(&self.tree),
            node: self.tree[*n].clone(),
        })
    }

    // pub fn data -> &SyntaxData
}
