//! various other useful types

use crate::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: Len,
    pub end: Len,
}

impl Span {
    pub fn intersects(self, other: Span) -> bool {
        self.start < other.end && other.start < self.end
    }
}
