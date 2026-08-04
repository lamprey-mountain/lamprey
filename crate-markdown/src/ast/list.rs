use std::str::FromStr;

use crate::ast::inline::Inline;
use crate::ast::{AstNode, impl_ast};
use crate::prelude::*;
use crate::tree::node::MarkdownLanguage;

#[derive(Debug)]
pub struct ListUnordered(SyntaxNode);
#[derive(Debug)]
pub struct ListOrdered(SyntaxNode);
#[derive(Debug)]
pub struct ListTasks(SyntaxNode);
#[derive(Debug)]
pub struct ListItemUnordered(SyntaxNode);
#[derive(Debug)]
pub struct ListItemOrdered(SyntaxNode);
#[derive(Debug)]
pub struct ListItemTasks(SyntaxNode);

impl_ast!(ListOrdered, NodeKind::Block(BlockKind::ListOrdered));
impl_ast!(ListUnordered, NodeKind::Block(BlockKind::ListUnordered));
impl_ast!(ListTasks, NodeKind::Block(BlockKind::ListTasks));

// TODO: is there a better way to do this?
impl_ast!(ListItemOrdered, NodeKind::Block(BlockKind::ListItem));
impl_ast!(ListItemUnordered, NodeKind::Block(BlockKind::ListItem));
impl_ast!(ListItemTasks, NodeKind::Block(BlockKind::ListItem));

// impl_ast!(ListItem, NodeKind::Block(BlockKind::ListItem));
// impl_ast!(
//     List,
//     NodeKind::Block(BlockKind::ListOrdered)
//         | NodeKind::Block(BlockKind::ListUnordered)
//         | NodeKind::Block(BlockKind::ListTasks)
// );

#[derive(Debug)]
pub enum List {
    Ordered(ListOrdered),
    Unordered(ListUnordered),
    Tasks(ListTasks),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListKind {
    Ordered,
    Unordered,
    Tasks,
}

impl ListKind {
    /// get a `BlockKind` for this kind of list
    pub fn block_kind(&self) -> BlockKind {
        match self {
            ListKind::Ordered => BlockKind::ListOrdered,
            ListKind::Unordered => BlockKind::ListUnordered,
            ListKind::Tasks => BlockKind::ListTasks,
        }
    }

    /// get a `NodeKind` for this kind of list
    pub fn node_kind(&self) -> NodeKind {
        NodeKind::Block(self.block_kind())
    }
}

// TODO: move this to util?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskListMark {
    /// ` ` - this task hasn't been started
    Unchecked,

    /// `-` - this task is in progress
    Working,

    /// `/` - this task has been cancelled
    Cancelled,

    /// `x` - this task is complete
    Complete,
}

impl FromStr for TaskListMark {
    // TODO: better error type?
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "" | " " => Ok(Self::Unchecked),
            "-" => Ok(Self::Working),
            "/" => Ok(Self::Cancelled),
            "x" => Ok(Self::Complete),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for TaskListMark {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TaskListMark::Unchecked => " ",
            TaskListMark::Working => "-",
            TaskListMark::Cancelled => "/",
            TaskListMark::Complete => "x",
        };
        write!(f, "{}", s)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for TaskListMark {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl AstNode for List {
    type Language = MarkdownLanguage;

    fn can_cast(kind: NodeKind) -> bool {
        matches!(
            kind,
            NodeKind::Block(
                BlockKind::ListUnordered | BlockKind::ListOrdered | BlockKind::ListTasks
            )
        )
    }

    fn cast(tn: SyntaxNode) -> Option<Self> {
        let kind = tn.kind();
        match kind {
            NodeKind::Block(BlockKind::ListUnordered) => {
                ListUnordered::cast(tn).map(Self::Unordered)
            }
            NodeKind::Block(BlockKind::ListOrdered) => ListOrdered::cast(tn).map(Self::Ordered),
            NodeKind::Block(BlockKind::ListTasks) => ListTasks::cast(tn).map(Self::Tasks),
            _ => None,
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            List::Ordered(l) => l.syntax(),
            List::Unordered(l) => l.syntax(),
            List::Tasks(l) => l.syntax(),
        }
    }
}

impl ListUnordered {
    pub fn kind(&self) -> ListKind {
        ListKind::Unordered
    }

    pub fn items(&self) -> impl Iterator<Item = ListItemUnordered> + '_ {
        self.0
            .children_with_tokens()
            .filter_map(|child| child.into_node().and_then(ListItemUnordered::cast))
    }
}

impl ListOrdered {
    pub fn kind(&self) -> ListKind {
        ListKind::Ordered
    }

    pub fn items(&self) -> impl Iterator<Item = ListItemOrdered> + '_ {
        self.0
            .children_with_tokens()
            .filter_map(|child| child.into_node().and_then(ListItemOrdered::cast))
    }
}

impl ListTasks {
    pub fn kind(&self) -> ListKind {
        ListKind::Tasks
    }

    pub fn items(&self) -> impl Iterator<Item = ListItemTasks> + '_ {
        self.0
            .children_with_tokens()
            .filter_map(|child| child.into_node().and_then(ListItemTasks::cast))
    }
}

impl ListItemUnordered {
    pub fn children(&self) -> impl Iterator<Item = Inline> + '_ {
        self.0
            .children_with_tokens()
            .filter(|c| {
                !matches!(
                    c.kind(),
                    NodeKind::Text(TextKind::ListPrefix | TextKind::Padding)
                )
            })
            .filter_map(Inline::cast)
    }
}

impl ListItemOrdered {
    pub fn number(&self) -> Option<u16> {
        // NOTE: do i want to use the user defined number or automatically increment? i *think* commonmark always autoincrements starting from the first list item's number.
        self.0
            .children_with_tokens()
            .find(|c| c.kind() == NodeKind::Text(TextKind::ListPrefix))
            .and_then(|c| c.to_string().trim_end_matches('.').parse().ok())
    }

    pub fn children(&self) -> impl Iterator<Item = Inline> + '_ {
        self.0
            .children_with_tokens()
            .filter(|c| {
                !matches!(
                    c.kind(),
                    NodeKind::Text(TextKind::ListPrefix | TextKind::Padding)
                )
            })
            .filter_map(Inline::cast)
    }
}

impl ListItemTasks {
    pub fn mark(&self) -> TaskListMark {
        self.0
            .children_with_tokens()
            .find(|c| c.kind() == NodeKind::Text(TextKind::TaskMark))
            .and_then(|c| c.to_string().as_str().trim().parse().ok())
            .unwrap_or(TaskListMark::Unchecked)
    }

    pub fn children(&self) -> impl Iterator<Item = Inline> + '_ {
        self.0
            .children_with_tokens()
            .filter(|c| {
                !matches!(
                    c.kind(),
                    NodeKind::Text(TextKind::TaskMark | TextKind::Padding)
                )
            })
            .filter_map(Inline::cast)
    }
}

// TODO: trait for various types of lists? nah, this is probably overkill...
// pub trait ListKind {
//     type Item;
// }
