use common::v2::types::MessageId;

/// a range of messages that are known to be loaded
#[derive(Debug, Clone, Copy)]
pub struct Range {
    /// the start of this interval (inclusive)
    pub start: MessageId,

    /// the end of this interval (inclusive)
    pub end: MessageId,

    /// whether this range is stale
    ///
    /// stale ranges can be used in the ui while loading, but MUST be replaced
    /// with fresh data from the server if it is received. eg. if paginating
    /// backwards, don't reuse stale ranges; instead fetch and write over them.
    pub stale: bool,
}

impl Range {
    /// construct a new [`Range`] containing a single (non stale) message id
    pub fn single(id: MessageId) -> Self {
        Self {
            start: id,
            end: id,
            stale: false,
        }
    }

    /// whether this range and another range are overlapping or adjacent (should merge)
    pub fn touches(&self, other: Range) -> bool {
        self.start <= other.end && other.start <= self.end
    }

    pub fn contains(&self, id: MessageId) -> bool {
        self.start <= id && id <= self.end
    }

    /// merge this range with another range
    pub fn merge(&self, other: Range) -> Range {
        Range {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            stale: self.stale && other.stale,
        }
    }
}
