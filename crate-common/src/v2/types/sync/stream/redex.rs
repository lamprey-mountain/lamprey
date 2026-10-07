//! a subscription to a redex
//!
//! allows receiving redex log and run events. a document subscription is
//! required to subscribe to the redex's content.

use lamprey_macros::record;

use crate::v2::types::{ChannelId, RedexId};

#[record]
pub struct Initial {
    pub channel_id: ChannelId,
    pub redex_id: RedexId,
}
