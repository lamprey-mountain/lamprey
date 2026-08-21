use lamprey_macros::record;

use crate::v1::types::{
    Message, MessageId, RoomMember, ThreadMember, User, search::common::SearchRequest,
};

#[record]
pub struct MessageSearchRequest {
    #[serde(flatten)]
    pub inner: SearchRequest,

    /// field to sort by
    #[serde(default)]
    pub sort_field: MessageSearchOrderField,

    /// whether to include results from nsfw channels
    pub include_nsfw: Option<bool>,
}

/// which field to order message search results by
#[record]
#[derive(Default, Copy, PartialEq, Eq)]
pub enum MessageSearchOrderField {
    /// sort by creation time
    #[default]
    Created,

    /// sort by relevancy
    Relevancy,
}

#[record]
pub struct MessageSearch {
    /// the ids of the matched messages
    pub results: Vec<MessageId>,

    /// all relevant messages (eg. messages that a result replied to)
    pub messages: Vec<Message>,

    /// the authors of the messages
    pub users: Vec<User>,

    /// threads the messages are in
    pub threads: Vec<crate::v1::types::Channel>,

    /// room members objects for each author, if they exist
    pub room_members: Vec<RoomMember>,

    /// relevant thread member objects
    ///
    /// - one for each (message author, thread) tuple
    /// - one for each thread the requesting user is a member of
    pub thread_members: Vec<ThreadMember>,

    /// whether there are more threads
    pub has_more: bool,

    /// approximate count of total results that match this query
    pub total: u64,

    /// current page cursor
    pub cursor: Option<String>,
}
