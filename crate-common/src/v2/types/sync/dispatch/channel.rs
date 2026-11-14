use lamprey_macros::record;

use super::invite::{InviteCreate, InviteDelete, InviteDeleteBulk, InviteUpdate};
use crate::{
    v1::types::{
        CalendarEventId, Channel, ChannelId, ChannelSeq, InviteTargetId, Message, TagId,
        ThreadMember,
        calendar::{CalendarEvent, CalendarOverwrite, CalendarParticipant},
        misc::Time,
        reaction::ReactionKey,
        tag::Tag,
        voice::{Call, VoiceState},
    },
    v2::types::{MessageId, MessageVerId, RoomId, UserId},
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
    MessageDelete(MessageDelete),
    MessageVersionDelete(MessageVersionDelete),
    MessageDeleteBulk(MessageDeleteBulk),
    MessageRemove(MessageRemove),
    MessageRestore(MessageRestore),
    ReactionCreate(ReactionCreate),
    ReactionDelete(ReactionDelete),
    ReactionDeleteKey(ReactionDeleteKey),
    ReactionDeleteAll(ReactionDeleteAll),
    ThreadMemberUpsert(ThreadMemberUpsert),
    CallCreate(CallCreate),
    CallUpdate(CallUpdate),
    CallDelete(CallDelete),
    CalendarEventCreate(CalendarEventCreate),
    CalendarEventUpdate(CalendarEventUpdate),
    CalendarEventDelete(CalendarEventDelete),
    CalendarOverwriteCreate(CalendarOverwriteCreate),
    CalendarOverwriteUpdate(CalendarOverwriteUpdate),
    CalendarOverwriteDelete(CalendarOverwriteDelete),
    CalendarRsvpCreate(CalendarRsvpCreate),
    CalendarRsvpDelete(CalendarRsvpDelete),
    CalendarInstanceRsvpCreate(CalendarInstanceRsvpCreate),
    CalendarInstanceRsvpDelete(CalendarInstanceRsvpDelete),
    TagCreate(TagCreate),
    TagUpdate(TagUpdate),
    TagDelete(TagDelete),
    VoiceStateUpdate(VoiceStateUpdate),
    InviteCreate(ChannelInviteCreate),
    InviteUpdate(ChannelInviteUpdate),
    InviteDelete(ChannelInviteDelete),
    InviteDeleteBulk(ChannelInviteDeleteBulk),
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

/// a message was deleted
#[record]
pub struct MessageDelete {
    pub channel_id: ChannelId,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_id: Option<RoomId>,

    pub message_id: MessageId,
    pub seq: ChannelSeq,
}

/// a specific version of a message was deleted
#[record]
pub struct MessageVersionDelete {
    pub channel_id: ChannelId,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_id: Option<RoomId>,

    pub message_id: MessageId,
    pub version_id: MessageVerId,
    pub seq: ChannelSeq,
}

/// multiple messages were deleted at once
#[record]
pub struct MessageDeleteBulk {
    pub channel_id: ChannelId,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_id: Option<RoomId>,

    pub message_ids: Vec<MessageId>,
    pub seq: ChannelSeq,
}

/// multiple messages were removed (hidden) at once
#[record]
pub struct MessageRemove {
    pub channel_id: ChannelId,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_id: Option<RoomId>,

    pub message_ids: Vec<MessageId>,
    pub seq: ChannelSeq,
}

/// multiple messages were restored after being removed
#[record]
pub struct MessageRestore {
    pub channel_id: ChannelId,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_id: Option<RoomId>,

    pub message_ids: Vec<MessageId>,
    pub seq: ChannelSeq,
}

/// a reaction was added to a message
#[record]
pub struct ReactionCreate {
    pub channel_id: ChannelId,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_id: Option<RoomId>,

    pub user_id: UserId,
    pub message_id: MessageId,
    pub key: ReactionKey,
    pub seq: ChannelSeq,
}

/// a specific user's reaction was removed from a message
#[record]
pub struct ReactionDelete {
    pub channel_id: ChannelId,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_id: Option<RoomId>,

    pub user_id: UserId,
    pub message_id: MessageId,
    pub key: ReactionKey,
    pub seq: ChannelSeq,
}

/// all reactions for a key were removed from a message
#[record]
pub struct ReactionDeleteKey {
    pub channel_id: ChannelId,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_id: Option<RoomId>,

    pub message_id: MessageId,
    pub key: ReactionKey,
    pub seq: ChannelSeq,
}

/// all reactions were removed from a message
#[record]
pub struct ReactionDeleteAll {
    pub channel_id: ChannelId,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_id: Option<RoomId>,

    pub message_id: MessageId,
    pub seq: ChannelSeq,
}

/// members were added to or removed from a thread
#[record]
pub struct ThreadMemberUpsert {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_id: Option<RoomId>,

    pub thread_id: ChannelId,
    pub added: Vec<ThreadMember>,
    pub removed: Vec<UserId>,
    pub seq: ChannelSeq,
}

/// a call was created
#[record]
pub struct CallCreate {
    pub call: Box<Call>,
    pub seq: ChannelSeq,
}

/// a call was updated
#[record]
pub struct CallUpdate {
    pub call: Box<Call>,
    pub seq: ChannelSeq,
}

/// a call was deleted
#[record]
pub struct CallDelete {
    pub channel_id: ChannelId,
    pub seq: ChannelSeq,
}

/// a calendar event was created
#[record]
pub struct CalendarEventCreate {
    pub event: Box<CalendarEvent>,
    pub seq: ChannelSeq,
}

/// a calendar event was updated
#[record]
pub struct CalendarEventUpdate {
    pub event: Box<CalendarEvent>,
    pub seq: ChannelSeq,
}

/// a calendar event was deleted
#[record]
pub struct CalendarEventDelete {
    pub channel_id: ChannelId,
    pub event_id: CalendarEventId,
    pub seq: ChannelSeq,
}

/// an overwrite was created for a calendar event instance
#[record]
pub struct CalendarOverwriteCreate {
    pub channel_id: ChannelId,
    pub event_id: CalendarEventId,
    pub overwrite: Box<CalendarOverwrite>,
    pub seq: ChannelSeq,
}

/// an overwrite was updated for a calendar event instance
#[record]
pub struct CalendarOverwriteUpdate {
    pub channel_id: ChannelId,
    pub event_id: CalendarEventId,
    pub overwrite: Box<CalendarOverwrite>,
    pub seq: ChannelSeq,
}

/// an overwrite was deleted for a calendar event instance
#[record]
pub struct CalendarOverwriteDelete {
    pub channel_id: ChannelId,
    pub event_id: CalendarEventId,
    pub instance_seq: u64,
    pub seq: ChannelSeq,
}

/// a user rsvped to a calendar event
#[record]
pub struct CalendarRsvpCreate {
    pub channel_id: ChannelId,
    pub event_id: CalendarEventId,
    pub participant: Box<CalendarParticipant>,
    pub seq: ChannelSeq,
}

/// a user removed their rsvp from a calendar event
#[record]
pub struct CalendarRsvpDelete {
    pub channel_id: ChannelId,
    pub event_id: CalendarEventId,
    pub user_id: UserId,
    pub seq: ChannelSeq,
}

/// a user rsvped to a calendar event instance
#[record]
pub struct CalendarInstanceRsvpCreate {
    pub channel_id: ChannelId,
    pub event_id: CalendarEventId,
    pub instance_seq: u64,
    pub participant: Box<CalendarParticipant>,
    pub seq: ChannelSeq,
}

/// a user removed their rsvp from a calendar event instance
#[record]
pub struct CalendarInstanceRsvpDelete {
    pub channel_id: ChannelId,
    pub event_id: CalendarEventId,
    pub instance_seq: u64,
    pub user_id: UserId,
    pub seq: ChannelSeq,
}

/// a tag was created
#[record]
pub struct TagCreate {
    pub tag: Box<Tag>,
    pub seq: ChannelSeq,
}

/// a tag was updated
#[record]
pub struct TagUpdate {
    pub tag: Box<Tag>,
    pub seq: ChannelSeq,
}

/// a tag was deleted
#[record]
pub struct TagDelete {
    pub channel_id: ChannelId,
    pub tag_id: TagId,
    pub seq: ChannelSeq,
}

/// a user's voice state was updated
#[record]
pub struct VoiceStateUpdate {
    pub user_id: UserId,
    pub channel_id: ChannelId,
    pub state: Option<Box<VoiceState>>,
    pub seq: ChannelSeq,
}

/// an invite in this channel was created
#[record]
pub struct ChannelInviteCreate {
    pub channel_id: ChannelId,
    #[serde(flatten)]
    pub invite: InviteCreate,
    pub seq: ChannelSeq,
}

/// an invite in this channel was updated
#[record]
pub struct ChannelInviteUpdate {
    pub channel_id: ChannelId,
    #[serde(flatten)]
    pub invite: InviteUpdate,
    pub seq: ChannelSeq,
}

/// an invite in this channel was deleted
#[record]
pub struct ChannelInviteDelete {
    pub channel_id: ChannelId,
    #[serde(flatten)]
    pub invite: InviteDelete,
    pub seq: ChannelSeq,
}

/// multiple invites in this channel were deleted
#[record]
pub struct ChannelInviteDeleteBulk {
    pub channel_id: ChannelId,
    #[serde(flatten)]
    pub invite: InviteDeleteBulk,
    pub seq: ChannelSeq,
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
            DispatchChannel::MessageDelete(a) => a.channel_id,
            DispatchChannel::MessageVersionDelete(a) => a.channel_id,
            DispatchChannel::MessageDeleteBulk(a) => a.channel_id,
            DispatchChannel::MessageRemove(a) => a.channel_id,
            DispatchChannel::MessageRestore(a) => a.channel_id,
            DispatchChannel::ReactionCreate(a) => a.channel_id,
            DispatchChannel::ReactionDelete(a) => a.channel_id,
            DispatchChannel::ReactionDeleteKey(a) => a.channel_id,
            DispatchChannel::ReactionDeleteAll(a) => a.channel_id,
            DispatchChannel::ThreadMemberUpsert(a) => a.thread_id,
            DispatchChannel::CallCreate(a) => a.call.channel_id,
            DispatchChannel::CallUpdate(a) => a.call.channel_id,
            DispatchChannel::CallDelete(a) => a.channel_id,
            DispatchChannel::CalendarEventCreate(a) => a.event.channel_id,
            DispatchChannel::CalendarEventUpdate(a) => a.event.channel_id,
            DispatchChannel::CalendarEventDelete(a) => a.channel_id,
            DispatchChannel::CalendarOverwriteCreate(a) => a.channel_id,
            DispatchChannel::CalendarOverwriteUpdate(a) => a.channel_id,
            DispatchChannel::CalendarOverwriteDelete(a) => a.channel_id,
            DispatchChannel::CalendarRsvpCreate(a) => a.channel_id,
            DispatchChannel::CalendarRsvpDelete(a) => a.channel_id,
            DispatchChannel::CalendarInstanceRsvpCreate(a) => a.channel_id,
            DispatchChannel::CalendarInstanceRsvpDelete(a) => a.channel_id,
            DispatchChannel::TagCreate(a) => a.tag.channel_id,
            DispatchChannel::TagUpdate(a) => a.tag.channel_id,
            DispatchChannel::TagDelete(a) => a.channel_id,
            DispatchChannel::VoiceStateUpdate(a) => a.channel_id,
            DispatchChannel::InviteCreate(a) => a.channel_id,
            DispatchChannel::InviteUpdate(a) => a.channel_id,
            DispatchChannel::InviteDelete(a) => a.channel_id,
            DispatchChannel::InviteDeleteBulk(a) => a.channel_id,
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
            DispatchChannel::MessageDelete(a) => a.room_id,
            DispatchChannel::MessageVersionDelete(a) => a.room_id,
            DispatchChannel::MessageDeleteBulk(a) => a.room_id,
            DispatchChannel::MessageRemove(a) => a.room_id,
            DispatchChannel::MessageRestore(a) => a.room_id,
            DispatchChannel::ReactionCreate(a) => a.room_id,
            DispatchChannel::ReactionDelete(a) => a.room_id,
            DispatchChannel::ReactionDeleteKey(a) => a.room_id,
            DispatchChannel::ReactionDeleteAll(a) => a.room_id,
            DispatchChannel::ThreadMemberUpsert(a) => a.room_id,
            DispatchChannel::CallCreate(a) => a.call.room_id,
            DispatchChannel::CallUpdate(a) => a.call.room_id,
            DispatchChannel::CallDelete(_) => None,
            DispatchChannel::CalendarEventCreate(a) => a.event.room_id,
            DispatchChannel::CalendarEventUpdate(a) => a.event.room_id,
            DispatchChannel::CalendarEventDelete(_) => None,
            DispatchChannel::CalendarOverwriteCreate(_) => None,
            DispatchChannel::CalendarOverwriteUpdate(_) => None,
            DispatchChannel::CalendarOverwriteDelete(_) => None,
            DispatchChannel::CalendarRsvpCreate(_) => None,
            DispatchChannel::CalendarRsvpDelete(_) => None,
            DispatchChannel::CalendarInstanceRsvpCreate(_) => None,
            DispatchChannel::CalendarInstanceRsvpDelete(_) => None,
            DispatchChannel::TagCreate(_) => None,
            DispatchChannel::TagUpdate(_) => None,
            DispatchChannel::TagDelete(_) => None,
            DispatchChannel::VoiceStateUpdate(a) => a.state.as_ref().and_then(|s| s.room_id),
            DispatchChannel::InviteCreate(a) => match &a.invite.target {
                InviteTargetId::Room { room_id, .. } => Some(*room_id),
                _ => None,
            },
            DispatchChannel::InviteUpdate(a) => match &a.invite.target {
                InviteTargetId::Room { room_id, .. } => Some(*room_id),
                _ => None,
            },
            DispatchChannel::InviteDelete(a) => match &a.invite.target {
                InviteTargetId::Room { room_id, .. } => Some(*room_id),
                _ => None,
            },
            DispatchChannel::InviteDeleteBulk(a) => match &a.invite.target {
                InviteTargetId::Room { room_id, .. } => Some(*room_id),
                _ => None,
            },
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
            DispatchChannel::MessageDelete(a) => Some(a.seq),
            DispatchChannel::MessageVersionDelete(a) => Some(a.seq),
            DispatchChannel::MessageDeleteBulk(a) => Some(a.seq),
            DispatchChannel::MessageRemove(a) => Some(a.seq),
            DispatchChannel::MessageRestore(a) => Some(a.seq),
            DispatchChannel::ReactionCreate(a) => Some(a.seq),
            DispatchChannel::ReactionDelete(a) => Some(a.seq),
            DispatchChannel::ReactionDeleteKey(a) => Some(a.seq),
            DispatchChannel::ReactionDeleteAll(a) => Some(a.seq),
            DispatchChannel::ThreadMemberUpsert(a) => Some(a.seq),
            DispatchChannel::CallCreate(a) => Some(a.seq),
            DispatchChannel::CallUpdate(a) => Some(a.seq),
            DispatchChannel::CallDelete(a) => Some(a.seq),
            DispatchChannel::CalendarEventCreate(a) => Some(a.seq),
            DispatchChannel::CalendarEventUpdate(a) => Some(a.seq),
            DispatchChannel::CalendarEventDelete(a) => Some(a.seq),
            DispatchChannel::CalendarOverwriteCreate(a) => Some(a.seq),
            DispatchChannel::CalendarOverwriteUpdate(a) => Some(a.seq),
            DispatchChannel::CalendarOverwriteDelete(a) => Some(a.seq),
            DispatchChannel::CalendarRsvpCreate(a) => Some(a.seq),
            DispatchChannel::CalendarRsvpDelete(a) => Some(a.seq),
            DispatchChannel::CalendarInstanceRsvpCreate(a) => Some(a.seq),
            DispatchChannel::CalendarInstanceRsvpDelete(a) => Some(a.seq),
            DispatchChannel::TagCreate(a) => Some(a.seq),
            DispatchChannel::TagUpdate(a) => Some(a.seq),
            DispatchChannel::TagDelete(a) => Some(a.seq),
            DispatchChannel::VoiceStateUpdate(a) => Some(a.seq),
            DispatchChannel::InviteCreate(a) => Some(a.seq),
            DispatchChannel::InviteUpdate(a) => Some(a.seq),
            DispatchChannel::InviteDelete(a) => Some(a.seq),
            DispatchChannel::InviteDeleteBulk(a) => Some(a.seq),
        }
    }
}

pub enum DispatchChannelInner {
    // NOTE: read state updates are user-scoped, see DispatchClient::ReadStateUpdate

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

    // ThreadMemberUpsert {
    //     room_id: Option<RoomId>,
    //     thread_id: ChannelId,

    //     /// members that were added to the thread
    //     added: Vec<ThreadMember>,

    //     /// members that were removed from the thread
    //     removed: Vec<UserId>,
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
