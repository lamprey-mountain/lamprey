// TODO: rip everything out and try again

use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use common::v1::types::{ChannelId, Message, MessageId, MessageSync, PaginationResponse};

/// messages in a channel
pub struct ChannelData {
    /// Global map of every message we know about in this channel.
    messages: BTreeMap<MessageId, Arc<Message>>,

    /// Metadata describing which segments of the BTreeMap are "connected".
    /// These MUST be kept sorted and disjoint.
    ranges: Vec<TimelineMetadata>,
}

/// a timeline is a sequence of contiguous messages
#[derive(Debug, Clone)]
pub struct TimelineMetadata {
    pub start: MessageId,
    pub end: MessageId,
    pub has_more_before: bool,
    pub has_more_after: bool,

    /// whether this range is stale
    ///
    /// stale ranges can be used in the ui while loading, but MUST be replaced
    /// with fresh data from the server if it is received. eg. if paginating
    /// backwards, don't reuse stale ranges; instead fetch and write over them.
    pub stale: bool,
}

impl TimelineMetadata {
    pub fn contains(&self, message_id: MessageId) -> bool {
        todo!("check if self.start <= message_id <= self.end")
    }
}

// pub struct Timeline<'a> {
//     pub timeline: &'a TimelineMetadata,
//     pub messages: Vec<&'a Message>,
// }

/// an update to the timeline
pub enum TimelineUpdate {
    /// a sync event from the websocket
    Sync(MessageSync),

    /// a response from the server
    FetchResult {
        task: FetchTask,
        response: PaginationResponse<Message>,
    },
}

pub struct QueryResult {
    pub channel_id: ChannelId,
    pub fragments: Vec<QueryResultFragment>,
    /// Does the server have even older messages than the first fragment?
    pub has_more_before: bool,
    /// Does the server have even newer messages than the last fragment?
    pub has_more_after: bool,
}

pub enum QueryResultFragment {
    /// A contiguous block of messages.
    Messages {
        items: Vec<Arc<Message>>,
        stale: bool,
    },
    /// A identified gap between two blocks or at a boundary.
    Gap(FetchTask),
}

#[derive(Debug, Clone)]
pub enum FetchTask {
    Backwards {
        from: MessageId,
        limit: u16,
    },
    Forwards {
        from: MessageId,
        limit: u16,
    },
    /// Fetching the newest messages in the channel.
    Live {
        limit: u16,
    },
    /// Fetching surrounding context for a specific message.
    Context {
        target: MessageId,
        limit: u16,
    },
}

#[derive(Debug, Clone)]
pub enum QueryAnchor {
    Latest,
    Around(MessageId),
    Before(MessageId),
    After(MessageId),
}

struct Messages {
    channels: HashMap<ChannelId, ChannelData>,
}

impl ChannelData {
    pub fn new(_channel_id: ChannelId) -> Self {
        Self {
            messages: BTreeMap::new(),
            ranges: Vec::new(),
        }
    }

    /// Internal: Add messages to the map and update ranges
    fn apply_fetch(&mut self, messages: Vec<Message>, task: FetchTask, has_more: bool) {
        if messages.is_empty() {
            // If the server returns empty, we update the existing range boundary
            self.mark_end_of_stream(&task);
            return;
        }

        // 1. Insert messages into the store
        let mut min_id = messages[0].id;
        let mut max_id = messages[0].id;

        for msg in messages {
            let id = msg.id;
            min_id = min_id.min(id);
            max_id = max_id.max(id);
            self.messages.insert(id, Arc::new(msg));
        }

        // 2. Create a temporary range for this fetch
        let (more_before, more_after) = match task {
            FetchTask::Backwards { .. } => (has_more, false),
            FetchTask::Forwards { .. } => (false, has_more),
            FetchTask::Live { .. } => (has_more, false), // Live is usually a backwards fetch from the tip
            FetchTask::Context { .. } => (has_more, has_more), // Simplification: context often has both
        };

        let new_range = TimelineMetadata {
            start: min_id,
            end: max_id,
            has_more_before: more_before,
            has_more_after: more_after,
            stale: false,
        };

        // 3. Reconcile
        self.merge_into_ranges(new_range);
    }

    fn merge_into_ranges(&mut self, mut new_range: TimelineMetadata) {
        let mut next_ranges = Vec::new();

        for existing in self.ranges.drain(..) {
            // Check for overlap or adjacency
            let overlapping =
                (new_range.start <= existing.end) && (existing.start <= new_range.end);

            // Adjacency check (if no 'more' flag exists between them, they are one timeline)
            let adjacent_after = new_range.end == existing.start && !new_range.has_more_after;
            let adjacent_before = existing.end == new_range.start && !existing.has_more_after;

            if overlapping || adjacent_after || adjacent_before {
                // Fold into new_range
                new_range.start = new_range.start.min(existing.start);
                new_range.end = new_range.end.max(existing.end);

                // If either is fresh, the result is fresh
                new_range.stale = new_range.stale && existing.stale;

                // Inherit boundary flags from the outermost ranges
                if existing.start < new_range.start {
                    new_range.has_more_before = existing.has_more_before;
                }
                if existing.end > new_range.end {
                    new_range.has_more_after = existing.has_more_after;
                }
            } else {
                next_ranges.push(existing);
            }
        }

        next_ranges.push(new_range);
        next_ranges.sort_by_key(|r| r.start);
        self.ranges = next_ranges;
    }

    fn mark_end_of_stream(&mut self, task: &FetchTask) {
        match task {
            FetchTask::Backwards { from, .. } => {
                if let Some(r) = self.ranges.iter_mut().find(|r| r.contains(*from)) {
                    r.has_more_before = false;
                }
            }
            FetchTask::Forwards { from, .. } => {
                if let Some(r) = self.ranges.iter_mut().find(|r| r.contains(*from)) {
                    r.has_more_after = false;
                }
            }
            _ => {}
        }
    }
}

impl Messages {
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
        }
    }

    pub fn query(&self, channel_id: ChannelId, anchor: QueryAnchor, limit: u16) -> QueryResult {
        let Some(data) = self.channels.get(&channel_id) else {
            return QueryResult {
                channel_id,
                fragments: vec![QueryResultFragment::Gap(FetchTask::Live { limit })],
                has_more_before: true,
                has_more_after: false,
            };
        };

        let mut fragments = Vec::new();
        let limit = limit as usize;

        match anchor {
            QueryAnchor::Latest => {
                // Find the "live" range (the one with no more messages after it)
                if let Some(live) = data.ranges.iter().find(|r| !r.has_more_after) {
                    let items: Vec<_> = data
                        .messages
                        .range(live.start..=live.end)
                        .rev() // Start from the newest
                        .take(limit)
                        .map(|(_, m)| Arc::clone(m))
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev() // Back to chronological
                        .collect();

                    fragments.push(QueryResultFragment::Messages {
                        items,
                        stale: live.stale,
                    });

                    // If we didn't fill the limit and the live range has more before it...
                    if fragments_len(&fragments) < limit && live.has_more_before {
                        fragments.insert(
                            0,
                            QueryResultFragment::Gap(FetchTask::Backwards {
                                from: live.start,
                                limit: (limit - fragments_len(&fragments)) as u16,
                            }),
                        );
                    }
                } else {
                    fragments.push(QueryResultFragment::Gap(FetchTask::Live {
                        limit: limit as u16,
                    }));
                }
            }
            QueryAnchor::Around(id) => {
                if let Some(range) = data.ranges.iter().find(|r| r.contains(id)) {
                    // Extract context
                    let half = limit / 2;
                    let items: Vec<_> = data
                        .messages
                        .range(..=id)
                        .rev()
                        .take(half)
                        .map(|(_, m)| Arc::clone(m))
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .chain(
                            data.messages
                                .range(id..)
                                .skip(1)
                                .take(half)
                                .map(|(_, m)| Arc::clone(m)),
                        )
                        .collect();

                    fragments.push(QueryResultFragment::Messages {
                        items,
                        stale: range.stale,
                    });
                } else {
                    fragments.push(QueryResultFragment::Gap(FetchTask::Context {
                        target: id,
                        limit: limit as u16,
                    }));
                }
            }
            _ => todo!("Implement Before/After anchors"),
        }

        QueryResult {
            channel_id,
            fragments,
            has_more_before: true, // Simplified
            has_more_after: false, // Simplified
        }
    }

    pub fn apply_event(&mut self, channel_id: ChannelId, event: TimelineUpdate) {
        let data = self
            .channels
            .entry(channel_id)
            .or_insert_with(|| ChannelData::new(channel_id));

        match event {
            TimelineUpdate::FetchResult { task, response } => {
                data.apply_fetch(response.items, task, response.has_more);
            }
            TimelineUpdate::Sync(sync) => match sync {
                MessageSync::MessageCreate { message } => {
                    // Optimized: try to append to the "live" range if it exists
                    let id = message.id;
                    data.messages.insert(id, Arc::new(message));

                    if let Some(live) = data.ranges.iter_mut().find(|r| !r.has_more_after) {
                        // If it's contiguous, just expand
                        if id >= live.end {
                            live.end = id;
                        } else {
                            // Message is older than live end? Re-merge
                            data.merge_into_ranges(TimelineMetadata {
                                start: id,
                                end: id,
                                has_more_before: true,
                                has_more_after: false,
                                stale: false,
                            });
                        }
                    } else {
                        // No live range? Create one
                        data.ranges.push(TimelineMetadata {
                            start: id,
                            end: id,
                            has_more_before: true,
                            has_more_after: false,
                            stale: false,
                        });
                    }
                }
                MessageSync::MessageDelete { message_id, .. } => {
                    data.messages.remove(&message_id);
                    // Note: We DO NOT shrink ranges on delete to maintain continuity
                }
                _ => {}
            },
        }
    }
}

fn fragments_len(frags: &[QueryResultFragment]) -> usize {
    frags
        .iter()
        .map(|f| match f {
            QueryResultFragment::Messages { items, .. } => items.len(),
            _ => 0,
        })
        .sum()
}
