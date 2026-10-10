use crate::types::analytics::{
    AnalyticsEvent, AnalyticsEventAggregatedType, AnalyticsEventDistinctType, ResourceAction,
};

/// utility to aggregate and anonymize analytics events
pub struct AnalyticsEventsAggregator {
    /// if a bucket has less than this many members, aggregate them into an "other" category
    min_bucket_size: u64,
}

impl AnalyticsEventsAggregator {
    /// aggregate and anonymize analytics events
    pub fn aggregate(_events: Vec<AnalyticsEvent>) -> Vec<AnalyticsEvent> {
        todo!()
    }
}

impl AnalyticsEvent {
    /// get the number of "added" resources
    pub fn stat_added(&self) -> u64 {
        match self {
            Self::Distinct(a) => a.inner.stat_added(),
            Self::Aggregated(a) => a.inner.stat_added(),
        }
    }

    /// get the number of "deleted" resources
    pub fn stat_deleted(&self) -> u64 {
        match self {
            Self::Distinct(a) => a.inner.stat_deleted(),
            Self::Aggregated(a) => a.inner.stat_deleted(),
        }
    }
}

impl AnalyticsEventDistinctType {
    pub fn stat_added(&self) -> u64 {
        match &self {
            Self::VoiceJoin { .. } => 1,
            Self::VoiceLeave { .. } => 0,
            Self::MemberJoin { .. } => 1,
            Self::MemberLeave { .. } => 0,
            Self::Room { action, .. } => action.stat_added(),
            Self::User { action, .. } => action.stat_added(),
            Self::Media { action, .. } => action.stat_added(),
            Self::MediaSize { bytes_added, .. } => *bytes_added,
            Self::Channel { action, .. } => action.stat_added(),
            Self::Auth { success, .. } => {
                if *success {
                    1
                } else {
                    0
                }
            }
        }
    }

    pub fn stat_deleted(&self) -> u64 {
        match &self {
            Self::VoiceJoin { .. } => 0,
            Self::VoiceLeave { .. } => 1,
            Self::MemberJoin { .. } => 0,
            Self::MemberLeave { .. } => 1,
            Self::Room { action, .. } => action.stat_deleted(),
            Self::User { action, .. } => action.stat_deleted(),
            Self::Media { action, .. } => action.stat_deleted(),
            Self::MediaSize { bytes_removed, .. } => *bytes_removed,
            Self::Channel { action, .. } => action.stat_deleted(),
            Self::Auth { .. } => 0,
        }
    }
}

impl AnalyticsEventAggregatedType {
    pub fn stat_added(&self) -> u64 {
        match self {
            Self::VoiceJoin { count, .. } => *count,
            Self::VoiceLeave { .. } => 0,
            Self::MemberJoin { count, .. } => *count,
            Self::MemberLeave { .. } => 0,
            Self::Room { count_created, .. } => *count_created,
            Self::User { count_created, .. } => *count_created,
            Self::Channel { count_created, .. } => *count_created,
            Self::MediaCountGlobal { count_created, .. } => *count_created,
            Self::MediaCountByRoom { count_created, .. } => *count_created,
            Self::MediaCountByUser { count_created, .. } => *count_created,
            Self::MediaSizeGlobal { bytes_created, .. } => *bytes_created,
            Self::MediaSizeByRoom { bytes_created, .. } => *bytes_created,
            Self::MediaSizeByUser { bytes_created, .. } => *bytes_created,
            Self::Message { count_created, .. } => *count_created,
        }
    }

    pub fn stat_deleted(&self) -> u64 {
        match self {
            Self::VoiceJoin { .. } => 0,
            Self::VoiceLeave { count, .. } => *count,
            Self::MemberJoin { .. } => 0,
            Self::MemberLeave { count, .. } => *count,
            Self::Room { count_deleted, .. } => *count_deleted,
            Self::User { count_deleted, .. } => *count_deleted,
            Self::Channel { count_deleted, .. } => *count_deleted,
            Self::MediaCountGlobal { count_deleted, .. } => *count_deleted,
            Self::MediaCountByRoom { count_deleted, .. } => *count_deleted,
            Self::MediaCountByUser { count_deleted, .. } => *count_deleted,
            Self::MediaSizeGlobal { bytes_deleted, .. } => *bytes_deleted,
            Self::MediaSizeByRoom { bytes_deleted, .. } => *bytes_deleted,
            Self::MediaSizeByUser { bytes_deleted, .. } => *bytes_deleted,
            Self::Message { count_deleted, .. } => *count_deleted,
        }
    }
}

impl ResourceAction {
    pub fn stat_added(&self) -> u64 {
        match self {
            Self::Create => 1,
            Self::Update => 0,
            Self::Delete => 0,
        }
    }

    pub fn stat_deleted(&self) -> u64 {
        match self {
            Self::Create => 0,
            Self::Update => 0,
            Self::Delete => 1,
        }
    }
}
