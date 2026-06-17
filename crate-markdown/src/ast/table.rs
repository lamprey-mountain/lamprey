use std::str::FromStr;

use crate::ast::inline::Inline;
use crate::ast::{AstNode, impl_ast};
use crate::prelude::*;
use crate::tree::node::MarkdownLanguage;

pub type TableIndex = u16;

#[derive(Debug, Clone)]
pub struct Table(SyntaxNode);

#[derive(Debug, Clone)]
pub struct TableRow {
    table: Table,
    node: SyntaxNode,
    index: TableIndex,
}

#[derive(Debug, Clone)]
pub struct TableCell {
    node: SyntaxNode,
    row: TableRow,
    column_idx: TableIndex,
}

#[derive(Debug, Clone)]
pub struct TableColumn {
    table: Table,
    index: TableIndex,
    header: TableCell,
    alignment: Alignment,
}

impl_ast!(Table, NodeKind::Block(BlockKind::Table));

impl AstNode for TableRow {
    type Language = MarkdownLanguage;

    fn can_cast(kind: NodeKind) -> bool {
        matches!(kind, NodeKind::Block(BlockKind::TableRow))
    }

    fn cast(node: SyntaxNode) -> Option<Self> {
        if !Self::can_cast(node.kind()) {
            return None;
        }
        let parent = node.parent()?;
        let index = parent
            .children_with_tokens()
            .filter_map(|child| child.into_node().and_then(TableRow::cast_raw))
            .position(|n| n == node)? as TableIndex;
        let table = Table::cast(node.parent().expect("table row must be in table"))
            .expect("table must be valid");
        Some(Self { node, index, table })
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.node
    }
}

impl TableRow {
    fn cast_raw(node: SyntaxNode) -> Option<SyntaxNode> {
        if matches!(node.kind(), NodeKind::Block(BlockKind::TableRow)) {
            Some(node)
        } else {
            None
        }
    }
}

impl AstNode for TableCell {
    type Language = MarkdownLanguage;

    fn can_cast(kind: NodeKind) -> bool {
        matches!(kind, NodeKind::Block(BlockKind::TableCell))
    }

    fn cast(node: SyntaxNode) -> Option<Self> {
        if !Self::can_cast(node.kind()) {
            return None;
        }

        let row_node = node.parent()?;
        let row = TableRow::cast(row_node)?;
        let column = row
            .node
            .children_with_tokens()
            .filter_map(|child| {
                child.into_node().and_then(|node| {
                    if matches!(node.kind(), NodeKind::Block(BlockKind::TableCell)) {
                        Some(node)
                    } else {
                        None
                    }
                })
            })
            .position(|n| n == node)? as TableIndex;
        Some(Self {
            node,
            row,
            column_idx: column,
        })
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.node
    }
}

/// how to align text in a table column
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Automatic, // how fancy shoud this be? align based on numeric decimal points?
    Left,
    Right,
    Center,
}

impl Table {
    /// Iterate over all rows in the table
    pub fn rows(&self) -> impl Iterator<Item = TableRow> + '_ {
        self.0
            .children_with_tokens()
            .filter_map(|child| child.into_node().and_then(TableRow::cast))
    }

    /// Get the header row of the table (the first row)
    pub fn header(&self) -> Option<TableRow> {
        self.rows().next()
    }

    /// Iterate over the body rows of the table (skipping header and alignment rows).
    pub fn body(&self) -> impl Iterator<Item = TableRow> + '_ {
        self.rows().skip(2)
    }

    /// Iterate over the columns of the table.
    pub fn columns(&self) -> impl Iterator<Item = TableColumn> + '_ {
        let mut rows = self.rows();
        let headers = rows.next().expect("TODO: better error handling");
        let alignments = rows.next().expect("TODO: better error handling");
        headers
            .cells()
            .zip(alignments.cells())
            .enumerate()
            .map(|(idx, (header, align))| TableColumn {
                table: self.clone(),
                index: idx as TableIndex,
                header,
                alignment: Alignment::from_str(&align.syntax().text().to_string())
                    .unwrap_or(Alignment::Automatic),
            })
            .collect::<Vec<_>>()
            .into_iter()
    }
}

impl TableColumn {
    /// get the table this belongs to
    #[inline]
    pub fn table(&self) -> Table {
        self.table.clone()
    }

    /// get index (position) of this column
    #[inline]
    pub fn index(&self) -> TableIndex {
        self.index
    }

    /// get the column's header cell
    #[inline]
    pub fn header(&self) -> TableCell {
        self.header.clone()
    }

    /// get the text alignment for this column
    #[inline]
    pub fn alignment(&self) -> Alignment {
        self.alignment
    }
}

impl TableRow {
    /// get the table this belongs to
    #[inline]
    pub fn table(&self) -> Table {
        self.table.clone()
    }

    /// get the index (position) of this row in the table
    #[inline]
    pub fn index(&self) -> TableIndex {
        self.index
    }

    /// iterate over table cells in this row
    pub fn cells(&self) -> impl Iterator<Item = TableCell> + '_ {
        self.node
            .children_with_tokens()
            .filter_map(|child| child.into_node().and_then(TableCell::cast))
    }
}

impl TableCell {
    /// get the table this belongs to
    #[inline]
    pub fn table(&self) -> Table {
        self.row.table()
    }

    /// get the row this cell belongs to
    #[inline]
    pub fn row(&self) -> TableRow {
        self.row.clone()
    }

    /// get the column this belongs to
    #[inline]
    pub fn column(&self) -> TableColumn {
        self.table()
            .columns()
            .nth(self.column_idx as usize)
            .expect("column must exist")
    }

    /// get the index (position) of this row in the column
    #[inline]
    pub fn index_row(&self) -> TableIndex {
        self.row.index()
    }

    /// get the index (position) of this column in the column
    #[inline]
    pub fn index_column(&self) -> TableIndex {
        self.column_idx
    }

    /// get the content of this table cell
    pub fn children(&self) -> impl Iterator<Item = Inline> + '_ {
        self.node
            .children_with_tokens()
            .filter_map(|child| Inline::cast(child))
    }
}

// TODO: write tests for this
impl FromStr for Alignment {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let left = s.starts_with(':');
        let right = s.ends_with(':');
        let inner = s
            .strip_prefix(':')
            .unwrap_or(s)
            .strip_suffix(':')
            .unwrap_or(s.strip_prefix(':').unwrap_or(s));

        if inner.is_empty() || !inner.chars().all(|c| c == '-') {
            return Err(());
        }

        Ok(match (left, right) {
            (true, true) => Self::Center,
            (true, false) => Self::Left,
            (false, true) => Self::Right,
            (false, false) => Self::Automatic,
        })
    }
}
