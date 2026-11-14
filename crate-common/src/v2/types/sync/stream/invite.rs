//! a subscription to a single invite
//!
//! used to preview an invite without access to its target

use lamprey_macros::record;

use crate::{
    util::registry::export_models,
    v1::types::{InviteCode, InviteTarget, User, misc::Time},
    v2::types::sync::stream::StreamProtocol,
};

pub struct Protocol;

#[record]
pub struct Initial {
    /// the invite to subscribe to
    pub code: InviteCode,
}

#[record]
#[serde(tag = "op")]
pub enum Command {
    // no commands
}

#[record]
#[serde(tag = "op")]
pub enum Event {
    /// the current or updated public preview of this invite
    Update(InvitePreview),

    /// this invite no longer exists
    Delete,

    /// confirmation that the client is now subscribed to the invite
    Subscribed,
}

/// the public preview of an invite
///
/// must not contain admin-only metadata such as `uses` or `max_uses`
#[record]
pub struct InvitePreview {
    pub code: InviteCode,
    pub target: InviteTarget,
    pub creator: User,
    pub created_at: Time,
    pub expires_at: Option<Time>,
    pub description: Option<String>,
    pub is_vanity: bool,
}

impl StreamProtocol for Protocol {
    type Initial = Initial;
    type Command = Command;
    type Event = Event;
}

export_models!(Initial, Command, Event, InvitePreview);
