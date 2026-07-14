use lamprey_macros::record;

use crate::v1::types::{Channel, Message, RoomMember, ThreadMemberMinimal, User};

/// Thread list
///
/// Response for listing threads
#[record]
pub struct ThreadList {
    pub threads: Vec<Channel>,
    pub total: u64,
    pub cursor: Option<String>,

    /// room members for each thread member in preview_members
    pub room_members: Vec<RoomMember>,

    /// users for each thread member in preview_members
    pub users: Vec<User>,
}

/// additional channel metadata for a thread
// TODO: add to struct Channel
#[record]
#[derive(Default)]
pub struct ThreadInfo {
    /// the first message sent in this thread
    ///
    /// for threads in `Forum`, `Forum2`, and `Ticket` channels, this is the "post" content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_message: Option<Box<Message>>,

    /// the last message sent in this thread
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_message: Option<Box<Message>>,

    /// a list of thread members
    // TODO: decide on a sensible max length
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preview_members: Vec<ThreadMemberMinimal>,
}

impl ThreadInfo {
    pub fn is_empty(&self) -> bool {
        self.first_message.is_none()
            && self.last_message.is_none()
            && self.preview_members.is_empty()
    }
}
