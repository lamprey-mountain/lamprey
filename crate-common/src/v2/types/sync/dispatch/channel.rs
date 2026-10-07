use lamprey_macros::record;

use crate::{
    v1::types::{Channel, ChannelId, ChannelSeq, Message, misc::Time},
    v2::types::{RoomId, UserId},
};

/// something happened in a channel
///
/// requires a subscription to the channel to receive
#[record]
pub struct DispatchChannel {
    pub channel_id: ChannelId,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_id: Option<RoomId>,

    /// the channel sync sequence number of this event
    ///
    /// used for offline sync. only populated if this dispatch incremented the sequence number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seq: Option<ChannelSeq>,

    #[serde(flatten)]
    pub inner: DispatchChannelInner,
}

#[record]
#[serde(tag = "type")]
pub enum DispatchChannelInner {
    // PERF: very minor thing, but some structs (eg. Channel or Message) have duplicate channel and room ids (with respect to DispatchChannel)
    // i'm not sure if theres a good way of avoiding this?
    /// a channel was created
    ChannelCreate { channel: Box<Channel> },

    /// a channel was updated
    ChannelUpdate { channel: Box<Channel> },

    // TODO: add?
    // ChannelDelete {
    //     channel_id: ChannelId,
    // },
    /// someone started typing in a channel
    ///
    /// requires a subscription in rooms
    ChannelTyping {
        user_id: UserId,
        until: Time,
        // #[serde(skip_serializing_if = "Option::is_none")]
        // room_id: Option<RoomId>,
        // NOTE: maybe include user, room member, channel/thread member objects?
    },

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
    /// a message was created
    // NOTE: maybe i should require a room subscription to receive message dispatches
    MessageCreate {
        /// the message itself
        message: Box<Message>,
        // NOTE: maybe i want to include resolved data either in the message or in the sync event itself
        // /// the room member of the author, if this was sent in a room
        // room_member: Option<Box<RoomMember>>,

        // /// the thread member of the author, if this was sent in a thread
        // thread_member: Option<Box<ThreadMember>>,

        // /// the user who sent this message
        // user: Box<User>,
    },

    MessageUpdate {
        message: Message,
        // /// the room member of the author, if this was sent in a room
        // room_member: Option<RoomMember>,

        // /// the thread member of the author, if this was sent in a thread
        // thread_member: Option<ThreadMember>,

        // /// the user who sent this message
        // user: User,
    },
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
