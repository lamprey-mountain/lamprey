use std::net::IpAddr;

use common::v1::types::{SessionId, UserId};

pub mod aggregate;
pub mod types;

pub use types::{
    AnalyticsEvent, AnalyticsEventAggregated, AnalyticsEventAggregatedType, AnalyticsEventDistinct,
    AnalyticsEventDistinctType,
};

#[derive(Debug, Clone)]
pub enum ResourceAction {
    Create,
    Update,
    Delete,
}

/// metadata for abuse monitoring
#[derive(Debug, Clone)]
pub struct AbuseMetadata {
    // unsure what would get logged if this is unknown?
    /// the ip address of the request that caused this event
    pub ip_addr: IpAddr,

    /// the user agent of the request that caused this event
    pub user_agent: String,

    /// the id of the session that caused this request
    ///
    /// may be None for unauthenticated requests
    pub session_id: Option<SessionId>,

    /// the id of the user that caused this request
    ///
    /// may be None for unauthenticated requests
    pub user_id: Option<UserId>,
    // TODO: unsure how useful this would be
    // /// ja3 fingerprint/hash
    // ///
    // /// tls fingerprint based on preferences during handshake
    // pub ja3_fingerprint: Option<String>,

    // /// ja4 fingerprint/hash
    // ///
    // /// like ja3, but with sorted client hello to reduce fingerpint cardinality
    // pub ja4_fingerprint: Option<String>,

    // // http stuff. probably redundant?
    // pub request_method: Option<String>,
    // pub request_path: Option<String>,
}
