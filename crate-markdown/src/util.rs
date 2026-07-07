//! various other useful types

use rowan::TextRange;

use crate::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Span {
    pub start: Len,
    pub end: Len,
}

impl Span {
    pub fn intersects(self, other: Span) -> bool {
        self.start < other.end && other.start < self.end
    }
}

impl From<(Len, Len)> for Span {
    fn from(value: (Len, Len)) -> Self {
        Self {
            start: value.0,
            end: value.1,
        }
    }
}

impl From<TextRange> for Span {
    fn from(value: TextRange) -> Self {
        Self {
            start: u32::from(value.start()) as Len,
            end: u32::from(value.end()) as Len,
        }
    }
}

impl From<Span> for TextRange {
    fn from(value: Span) -> Self {
        TextRange::new((value.start as u32).into(), (value.end as u32).into())
    }
}
