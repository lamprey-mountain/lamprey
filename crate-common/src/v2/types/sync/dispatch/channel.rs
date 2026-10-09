use lamprey_macros::record;

use crate::{
    v1::types::{Channel, ChannelId, ChannelSeq, Message, misc::Time},
    v2::types::{RoomId, UserId},
};

/// something happened in a channel
///
/// requires a subscription to the channel to receive
#[record]
#[serde(tag = "type")]
pub enum DispatchChannel {
    ChannelCreate(ChannelCreate),
    ChannelUpdate(ChannelUpdate),
    ChannelTyping(ChannelTyping),
    MessageCreate(MessageCreate),
    MessageUpdate(MessageUpdate),
}

/// a channel was created
#[record]
pub struct ChannelCreate {
    pub channel: Box<Channel>,
    pub seq: ChannelSeq,
}

/// a channel was updated
#[record]
pub struct ChannelUpdate {
    pub channel: Box<Channel>,
    pub seq: ChannelSeq,
}

// TODO: add ChannelDelete?

/// someone started typing in a channel
// NOTE: maybe include user, room member, channel/thread member objects?
#[record]
pub struct ChannelTyping {
    pub channel_id: ChannelId,
    pub user_id: UserId,
    pub until: Time,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_id: Option<RoomId>,
}

/// a message was created
// NOTE: maybe i should require a room subscription to receive message dispatches
#[record]
pub struct MessageCreate {
    pub message: Box<Message>,
    pub seq: ChannelSeq,
    // NOTE: maybe i want to include resolved data either in the message or in the sync event itself
    // /// the room member of the author, if this was sent in a room
    // room_member: Option<Box<RoomMember>>,

    // /// the thread member of the author, if this was sent in a thread
    // thread_member: Option<Box<ThreadMember>>,

    // /// the user who sent this message
    // user: Box<User>,
}

#[record]
pub struct MessageUpdate {
    pub message: Message,
    pub seq: ChannelSeq,
    // /// the room member of the author, if this was sent in a room
    // room_member: Option<RoomMember>,

    // /// the thread member of the author, if this was sent in a thread
    // thread_member: Option<ThreadMember>,

    // /// the user who sent this message
    // user: User,
}

impl DispatchChannel {
    /// id of the channel this event happened in
    pub fn channel_id(&self) -> ChannelId {
        match self {
            DispatchChannel::ChannelCreate(a) => a.channel.id,
            DispatchChannel::ChannelUpdate(a) => a.channel.id,
            DispatchChannel::ChannelTyping(a) => a.channel_id,
            DispatchChannel::MessageCreate(a) => a.message.channel_id,
            DispatchChannel::MessageUpdate(a) => a.message.channel_id,
        }
    }

    /// id of the room this event happened in
    pub fn room_id(&self) -> Option<RoomId> {
        match self {
            DispatchChannel::ChannelCreate(a) => a.channel.room_id,
            DispatchChannel::ChannelUpdate(a) => a.channel.room_id,
            DispatchChannel::ChannelTyping(a) => a.room_id,
            DispatchChannel::MessageCreate(a) => a.message.room_id,
            DispatchChannel::MessageUpdate(a) => a.message.room_id,
        }
    }

    /// the channel sync sequence number of this event
    pub fn seq(&self) -> Option<ChannelSeq> {
        match self {
            DispatchChannel::ChannelCreate(a) => Some(a.seq),
            DispatchChannel::ChannelUpdate(a) => Some(a.seq),
            DispatchChannel::ChannelTyping(_) => None,
            DispatchChannel::MessageCreate(a) => Some(a.seq),
            DispatchChannel::MessageUpdate(a) => Some(a.seq),
        }
    }
}

pub enum DispatchChannelInner {
    // NOTE: is ChannelAck a channel or a user event?
    // /// read receipt update
    // ChannelAck {
    //     user_id: UserId,
    //     channel_id: ChannelId,
    //     message_id: MessageId,
    //     version_id: MessageVerId,
    // },

    // unsure about these
    // // ThreadCreate {
    // //     thread: Box<Channel>,
    // // },

    // // ThreadUpdate {
    // //     thread: Box<Channel>,
    // // },

    // // ThreadDelete {
    // //     thread_id: ChannelId,
    // // },

    // MessageDelete {
    //     message_id: MessageId,
    // },

    // MessageVersionDelete {
    //     message_id: MessageId,
    //     version_id: MessageVerId,
    // },

    // /// delete multiple messages at once
    // MessageDeleteBulk {
    //     channel_id: ChannelId,
    //     message_ids: Vec<MessageId>,
    // },

    // MessageRemove {
    //     channel_id: ChannelId,
    //     message_ids: Vec<MessageId>,
    // },

    // MessageRestore {
    //     channel_id: ChannelId,

    //     // TODO: remove `message_ids`
    //     message_ids: Vec<MessageId>,
    //     // TODO: add `messages`
    //     // messages: Vec<Message>,
    // },

    // ThreadMemberUpsert {
    //     room_id: Option<RoomId>,
    //     thread_id: ChannelId,

    //     /// members that were added to the thread
    //     added: Vec<ThreadMember>,

    //     /// members that were removed from the thread
    //     removed: Vec<UserId>,
    // },

    // ReactionCreate {
    //     user_id: UserId,
    //     channel_id: ChannelId,
    //     message_id: MessageId,
    //     key: ReactionKey,
    // },

    // /// remove one specific emoji on a message
    // ReactionDelete {
    //     user_id: UserId,
    //     channel_id: ChannelId,
    //     message_id: MessageId,
    //     key: ReactionKey,
    // },

    // /// remove all reactions for a reaction key on a message
    // ReactionDeleteKey {
    //     channel_id: ChannelId,
    //     message_id: MessageId,
    //     key: ReactionKey,
    // },

    // /// remove all reactions on a message
    // ReactionDeleteAll {
    //     channel_id: ChannelId,
    //     message_id: MessageId,
    // },

    // TagCreate {
    //     tag: Tag,
    // },

    // TagUpdate {
    //     tag: Tag,
    // },

    // TagDelete {
    //     channel_id: ChannelId,
    //     tag_id: TagId,
    // },

    // RatelimitUpdate {
    //     channel_id: ChannelId,
    //     user_id: UserId,
    //     slowmode_thread_expire_at: Option<Time>,
    //     slowmode_message_expire_at: Option<Time>,
    // },

    // /// streaming message response
    // ///
    // /// when a user initially connects, a delta for all active flumes is sent
    // /// containing the full content of each flume (apply to empty component to
    // /// get current flume state)
    // FlumeDelta {
    //     channel_id: ChannelId,
    //     message_id: MessageId,
    //     delta: FlumeDelta,
    // },

    // CallCreate {
    //     call: Call,
    // },

    // CallUpdate {
    //     call: Call,
    // },

    // CallDelete,
}

pub enum DispatchCalendar {
    // CalendarEventCreate {
    //     event: CalendarEvent,
    // },

    // CalendarEventUpdate {
    //     event: CalendarEvent,
    // },

    // CalendarEventDelete {
    //     channel_id: ChannelId,
    //     event_id: CalendarEventId,
    // },

    // CalendarOverwriteCreate {
    //     channel_id: ChannelId,
    //     overwrite: CalendarOverwrite,
    // },

    // CalendarOverwriteUpdate {
    //     channel_id: ChannelId,
    //     overwrite: CalendarOverwrite,
    // },

    // CalendarOverwriteDelete {
    //     channel_id: ChannelId,
    //     event_id: CalendarEventId,
    //     seq: u64,
    // },

    // CalendarRsvpCreate {
    //     channel_id: ChannelId,
    //     event_id: CalendarEventId,
    //     participant: CalendarEventParticipant,
    // },

    // CalendarRsvpDelete {
    //     channel_id: ChannelId,
    //     event_id: CalendarEventId,
    //     user_id: UserId,
    // },

    // CalendarOverwriteRsvpCreate {
    //     channel_id: ChannelId,
    //     event_id: CalendarEventId,
    //     seq: u64,
    //     participant: CalendarEventParticipant,
    // },

    // CalendarOverwriteRsvpDelete {
    //     channel_id: ChannelId,
    //     event_id: CalendarEventId,
    //     seq: u64,
    //     user_id: UserId,
    // },
}

pub enum DispatchDocument {
    // DocumentTagCreate {
    //     channel_id: ChannelId,
    //     tag: DocumentTag,
    // },

    // DocumentTagUpdate {
    //     channel_id: ChannelId,
    //     tag: DocumentTag,
    // },

    // DocumentTagDelete {
    //     channel_id: ChannelId,
    //     branch_id: DocumentBranchId,
    //     tag_id: DocumentTagId,
    // },

    // DocumentBranchCreate {
    //     branch: DocumentBranch,
    // },

    // DocumentBranchUpdate {
    //     branch: DocumentBranch,
    // },

    // // NOTE: currently unused, as branches are marked as closed/merged rather than deleted
    // // how do i want to handle branch deletions? i want to clean up old editing contexts. maybe once closed/merged, make branches readonly and delete the associated editing context
    // DocumentBranchDelete {
    //     channel_id: ChannelId,
    //     branch_id: DocumentBranchId,
    // },
}

pub enum DispatchScript {
    // ScriptCreate {
    //     script: Redex,
    // },

    // ScriptUpdate {
    //     script: Redex,
    // },

    // ScriptDelete {
    //     channel_id: ChannelId,
    //     redex_id: RedexId,
    // },

    // ScriptVersionCreate {
    //     channel_id: ChannelId,
    //     redex_id: RedexId,
    //     version: RedexVersion,
    // },

    // // eg. when a script's inputs are done being processed
    // ScriptVersionUpdate {
    //     channel_id: ChannelId,
    //     redex_id: RedexId,
    //     version: RedexVersion,
    // },

    // ScriptVersionDelete {
    //     channel_id: ChannelId,
    //     redex_id: RedexId,
    //     version_id: RedexVerId,
    // },

    // ScriptRunCreate {
    //     channel_id: ChannelId,
    //     run: Eval,
    // },

    // ScriptRunUpdate {
    //     channel_id: ChannelId,
    //     run: Eval,
    // },

    // /// receive logs from a script
    // ///
    // /// must be subscribed to the script
    // ScriptLogCreate {
    //     channel_id: ChannelId,
    //     run_id: EvalId,
    //     entry: EvalLogEntry,
    // },

    // /// metrics for the channel a script is in
    // ///
    // /// must be subscribed to the script
    // // HACK: this api design is a bit dubious, will clean it up later
    // ScriptChannelMetrics {
    //     channel_id: ChannelId,
    //     memory_usage: usize,
    // },
}
