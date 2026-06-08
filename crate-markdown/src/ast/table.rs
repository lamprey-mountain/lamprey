use std::str::FromStr;

use crate::prelude::*;

pub struct Table(SyntaxNode);
pub struct TableRow(SyntaxNode);
pub struct TableCell(SyntaxNode);

pub struct TableColumn {
    header: SyntaxNode,
    alignment: Alignment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Automatic,
    Left,
    Right,
    Center,
}

impl Table {
    // fn columns(&self) -> iterator over table columns
    // fn rows(&self) -> iterator over table rows
}

impl TableColumn {
    // fn header(&self) -> iter over column header children
    // fn alignment(&self) -> alignment
}

impl TableRow {
    // fn cells(&self) -> iter over children
}

impl TableCell {
    // fn children(&self) -> iter over children
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
