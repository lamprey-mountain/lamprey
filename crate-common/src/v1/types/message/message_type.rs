use core::fmt;

use lamprey_macros::record;

#[cfg(feature = "feat_e2ee")]
use crate::v1::types::MessageEncrypted;
use crate::v1::types::{
    AuditLogEntry, ChannelId, DocumentBranchId, MediaId, MessageDefaultMarkdown, MessageId, RoomId,
    TagId, UserId,
    automod::{AutomodAction, AutomodMatches, AutomodRuleSummary},
    document::{DocumentRevisionId, DocumentTag},
    misc::Time,
    moderation::Report,
};

// NOTE: utoipa doesnt seem to like #[deprecated] here
#[record]
#[serde(tag = "type")]
pub enum MessageType {
    /// a basic message, using markdown
    // TODO: rename to Default?
    DefaultMarkdown(MessageDefaultMarkdown),

    /// a thread initial message, using markdown
    ThreadInitial(MessageDefaultMarkdown),

    /// an encrypted message
    #[cfg(feature = "feat_e2ee")]
    Encrypted(MessageEncrypted),

    /// a message was pinned
    MessagePinned(MessagePin),

    #[cfg(feature = "feat_message_move")]
    /// (TODO) one or more messages were moved
    MessagesMoved(MessagesMoved),

    /// a thread member was added to the thread or group dm
    MemberAdd(MessageMember),

    /// a thread member was removed from the thread or group dm
    MemberRemove(MessageMember),

    /// a room member joined the room
    MemberJoin,

    /// a call was started in a dm or gdm
    Call(MessageCall),

    /// this thread was renamed
    ChannelRename(MessageChannelRename),

    /// (TODO) someone mentioned this thread
    // TODO: rename to ChannelPingback
    // needs some sort of antispam system. again, see github.
    // doesnt necessarily reference a thread in the same room, but usually should
    // maybe don't include in log?
    ChannelPingback(MessageChannelPingback),

    /// this thread was moved
    ChannelMoved(MessageChannelMoved),

    /// this thread was tagged
    ChannelTagged(MessageChannelTagged),

    /// The channel's icon was changed
    ChannelIcon(MessageChannelIcon),
    // /// (TODO) receive announcement threads from this room
    // // but where does this get sent to???
    // RoomFollowed(MessageRoomFollowed),
    /// A thread was created from a message
    ThreadCreated(MessageThreadCreated),

    // /// (TODO) interact with a bot, uncertain if i'll go this route
    // BotCommand(MessageBotCommand),
    /// the result of an automod execution
    AutomodExecution(MessageAutomodExecution),
    // /// (TODO) implement a reporting system? uncertain (reports are certain, but reports-as-messages vs as-threads idk)
    // // #[deprecated = "reports will be impl'd as threads"]
    // ModerationReport(MessageModerationReport),
    /// someone nudged you!
    Nudge,

    /// a document tag was created
    DocumentTag(MessageDocumentTag),

    /// summary of recent document edits
    DocumentEdits(MessageDocumentEdits),

    /// another branch was merged into this branch
    // NOTE: i'll reuse ThreadCreated for when branches are created/forked
    DocumentMerged(MessageDocumentMerged),
}

// impl fmt::Display for MessageType {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         match self {
//             MessageType::DefaultMarkdown(_) => write!(f, "DefaultMarkdown"),
//             #[cfg(feature = "feat_e2ee")]
//             MessageType::Encrypted(_) => write!(f, "Encrypted"),
//             MessageType::MessagePinned(_) => write!(f, "MessagePinned"),
//             #[cfg(feature = "feat_message_move")]
//             MessageType::MessagesMoved(_) => write!(f, "MessagesMoved"),
//             MessageType::MemberAdd(_) => write!(f, "MemberAdd"),
//             MessageType::MemberRemove(_) => write!(f, "MemberRemove"),
//             MessageType::MemberJoin => write!(f, "MemberJoin"),
//             MessageType::Call(_) => write!(f, "Call"),
//             MessageType::ChannelRename(_) => write!(f, "ChannelRename"),
//             MessageType::ChannelPingback(_) => write!(f, "ChannelPingback"),
//             MessageType::ChannelMoved(_) => write!(f, "ChannelMoved"),
//             MessageType::ChannelIcon(_) => write!(f, "ChannelIcon"),
//             MessageType::ThreadCreated(_) => write!(f, "ThreadCreated"),
//             MessageType::AutomodExecution(_) => write!(f, "AutomodExecution"),
//         }
//     }
// }

#[record]
#[derive(Default, PartialEq, Eq)]
pub struct MessageCall {
    /// when the call ended. is None if the call is still going.
    pub ended_at: Option<Time>,

    /// the people who joined the call
    #[schema(min_length = 0, max_length = 128)]
    pub participants: Vec<UserId>,
}

/// Information about a message being pinned
#[record]
pub struct MessagePin {
    pub pinned_message_id: MessageId,
}

/// Information about an auto moderation execution
#[record]
pub struct MessageAutomodExecution {
    /// the rules that were triggered
    #[schema(min_length = 0, max_length = 32)]
    pub rules: Vec<AutomodRuleSummary>,

    /// the actions that were executed
    #[schema(min_length = 0, max_length = 32)]
    pub actions: Vec<AutomodAction>,

    /// the content that was matched
    // TODO: skip serializing if is none
    pub matches: Option<AutomodMatches>,

    /// the user who triggered this execution
    pub user_id: UserId,

    /// the id of the channel where the message was sent, is None if this is not a message
    // NOTE: this is only populated if the target is a mesage
    // TODO: design thread and other automod target types
    // TODO: skip serializing if is none
    pub channel_id: Option<ChannelId>,

    /// if the message wasn't blocked, this is the id of it
    pub flagged_message_id: Option<MessageId>,
    // pub completed: Option<AutomodAlertCompleted>,
}

// struct AutomodAlertCompleted {
//     /// the user who completed this alert
//     user_id: UserId,

//     /// when this alert was completed at
//     completed_at: Time,
// }

/// Information about a thread being renamed
#[record]
pub struct MessageChannelRename {
    #[serde(alias = "new")]
    pub name_new: String,

    #[serde(alias = "old")]
    pub name_old: String,
}

/// Information about a thread being moved
#[record]
pub struct MessageChannelMoved {
    pub parent_id_old: Option<ChannelId>,
    pub parent_id_new: Option<ChannelId>,
}

/// Information about a thread's tags being changed
#[record]
pub struct MessageChannelTagged {
    /// the tags that were added to this thread
    pub tags_added: Vec<TagId>,

    /// the tags that were removed from this thread
    pub tags_removed: Vec<TagId>,
}

/// Information about a thread being created
#[record]
pub struct MessageThreadCreated {
    /// the message this thread was created from
    pub source_message_id: Option<MessageId>,

    /// the id of the thread that was created
    // FIXME: this shouldn't be an Option
    pub thread_id: Option<ChannelId>,
}

/// Information about the pingback
#[record]
pub struct MessageChannelPingback {
    pub source_room_id: RoomId,
    pub source_channel_id: ChannelId,
    pub source_user_id: UserId,
}

/// Information about a channel icon change
#[record]
pub struct MessageChannelIcon {
    pub icon_id_old: Option<MediaId>,
    pub icon_id_new: Option<MediaId>,
}

#[cfg(feature = "feat_message_move")]
/// Information about one or more messages being moved between threads
/// probably want this being sent in both the source and target threads, maybe
/// with a bit of different styling depending on whether its source/target
#[record]
pub struct MessagesMoved {
    // do messages keep their ids when being moved?
    pub start_id: MessageId,
    pub end_id: MessageId,
    pub source_id: ChannelId,
    pub target_id: ChannelId,
    pub reason: Option<String>,
}

/// Information about a member being added or removed from a thread
#[record]
pub struct MessageMember {
    pub target_user_id: UserId,
}

/// Following a room and will receive announcement posts from it
#[record]
pub struct MessageRoomFollowed {
    pub thread_id: ChannelId,
    pub reason: Option<String>,
}

// TODO: remove
/// audit log entries as a message (builtin moderation logging?)
#[record]
pub struct MessageModerationLog {
    pub audit_log_entry: AuditLogEntry,
}

/// a report that moderators should look at
#[record]
pub struct MessageModerationReport {
    pub report: Report,
}

/// a bot command
#[record]
pub struct MessageBotCommand {
    pub command_id: String,
}

#[record]
pub struct MessageDocumentTag {
    /// the tag that was created
    pub tag: DocumentTag,
}

#[record]
pub struct MessageDocumentEdits {
    /// summary of recent edits
    // NOTE: probably will be autogenerated via llm?
    // NOTE: im not sure how patchwork determines when to include a summary?
    pub summary: String,

    /// the first revision included in this summary
    pub revision_start: DocumentRevisionId,

    /// the last revision included in this summary
    pub revision_end: DocumentRevisionId,
}

#[record]
pub struct MessageDocumentMerged {
    /// the id of the branch that was merged into this branch
    pub branch_id: DocumentBranchId,
}

impl fmt::Display for MessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // NOTE: i probably want a better Display impl than using fmt_tantivy
        self.fmt_tantivy(f)
    }
}

// TODO: macro or something for is_foo methods?
// maybe make message type a trait { is_deletable, ... }, MessageType lists all known message types?
impl MessageType {
    // NOTE: check if i can use strum here
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageType::DefaultMarkdown(_) => "DefaultMarkdown",
            MessageType::ThreadInitial(_) => "ThreadInitial",
            #[cfg(feature = "feat_e2ee")]
            MessageType::Encrypted(_) => "Encrypted",
            MessageType::MessagePinned(_) => "MessagePinned",
            #[cfg(feature = "feat_message_move")]
            MessageType::MessagesMoved(_) => "MessagesMoved",
            MessageType::MemberAdd(_) => "MemberAdd",
            MessageType::MemberRemove(_) => "MemberRemove",
            MessageType::MemberJoin => "MemberJoin",
            MessageType::Call(_) => "Call",
            MessageType::ChannelRename(_) => "ChannelRename",
            MessageType::ChannelPingback(_) => "ChannelPingback",
            MessageType::ChannelMoved(_) => "ChannelMoved",
            MessageType::ChannelTagged(_) => "ChannelTagged",
            MessageType::ChannelIcon(_) => "ChannelIcon",
            MessageType::ThreadCreated(_) => "ThreadCreated",
            MessageType::AutomodExecution(_) => "AutomodExecution",
            MessageType::Nudge => "Nudge",
            MessageType::DocumentTag(_) => "DocumentTag",
            MessageType::DocumentEdits(_) => "DocumentEdits",
            MessageType::DocumentMerged(_) => "DocumentMerged",
        }
    }

    // TODO: return if this is deletable by sender, not deletable by sender, or not deletable at all (even by mods)
    pub fn is_deletable(&self) -> bool {
        match self {
            MessageType::DefaultMarkdown(_) => true,

            // the initial thread message can be stripped but not actually deleted
            MessageType::ThreadInitial(_) => true,

            #[cfg(feature = "feat_e2ee")]
            MessageType::Encrypted(_) => true,
            #[cfg(feature = "feat_message_forwarding")]
            MessageType::Forward(_) => true,
            MessageType::MessagePinned(_) => true,
            MessageType::MemberAdd(_) => false,
            MessageType::MemberRemove(_) => false,
            MessageType::MemberJoin => true,
            MessageType::ChannelRename(_) => false,
            MessageType::ChannelPingback(_) => true,
            MessageType::ChannelIcon(_) => false,
            #[cfg(feature = "feat_message_move")]
            MessageType::MessagesMoved(_) => false,
            MessageType::Call(_) => false,
            MessageType::ThreadCreated(_) => false,
            MessageType::ChannelMoved(_) => false,
            MessageType::ChannelTagged(_) => false,
            MessageType::DocumentTag(_) => true,
            MessageType::DocumentEdits(_) => true,
            MessageType::DocumentMerged(_) => false,
            MessageType::Nudge => false,

            // NOTE: this should require the MessageDelete permission
            MessageType::AutomodExecution(_) => true,
        }
    }

    pub fn is_editable(&self) -> bool {
        matches!(
            self,
            MessageType::DefaultMarkdown(_) | MessageType::ThreadInitial(_)
        )
    }

    /// if threads can be created from this message
    pub fn is_threadable(&self) -> bool {
        matches!(self, MessageType::DefaultMarkdown(_))
    }

    pub fn is_movable(&self) -> bool {
        matches!(self, MessageType::DefaultMarkdown(_))
    }

    /// if this will be returned in the thread activity route
    // NOTE: update these queries when is_activity is updated
    // crate-backend-data-postgres/sql/message_activity_paginate.sql
    // crate-backend-data-postgres/sql/message_activity_count.sql
    pub fn is_activity(&self) -> bool {
        match self {
            MessageType::DefaultMarkdown(_) => false,
            MessageType::ThreadInitial(_) => false,
            #[cfg(feature = "feat_e2ee")]
            MessageType::Encrypted(_) => false,
            #[cfg(feature = "feat_message_forwarding")]
            MessageType::Forward(_) => false,
            MessageType::MessagePinned(_) => true,
            MessageType::MemberAdd(_) => true,
            MessageType::MemberRemove(_) => true,
            MessageType::MemberJoin => false,
            MessageType::ChannelRename(_) => true,
            MessageType::ChannelPingback(_) => true,
            MessageType::ChannelIcon(_) => true,
            MessageType::ChannelMoved(_) => true,
            MessageType::ChannelTagged(_) => true,
            #[cfg(feature = "feat_message_move")]
            MessageType::MessagesMoved(_) => false,
            MessageType::Call(_) => false,
            MessageType::ThreadCreated(_) => true,
            MessageType::AutomodExecution(_) => false,
            MessageType::DocumentTag(_) => true,
            MessageType::DocumentMerged(_) => true,
            MessageType::DocumentEdits(_) => false,
            MessageType::Nudge => false,
        }
    }

    // TODO: remove this, move to tantivy? or rename?
    /// format a message for tantivy search indexing
    pub fn fmt_tantivy(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: include ids (eg. user id in MemberAdd, or message id in MessagePinned)
        match self {
            MessageType::DefaultMarkdown(m) | MessageType::ThreadInitial(m) => {
                if let Some(content) = &m.content {
                    write!(f, "{}", content)
                } else {
                    write!(f, "")
                }
            }
            #[cfg(feature = "feat_e2ee")]
            MessageType::Encrypted(e) => {
                write!(f, "encrypted ({} byte ciphertext)", e.ciphertext.len())
            }
            MessageType::MessagePinned(_) => {
                write!(f, "message pinned")
            }
            MessageType::MemberAdd(_) => {
                write!(f, "member add")
            }
            MessageType::MemberRemove(_) => {
                write!(f, "member removed")
            }
            MessageType::MemberJoin => {
                write!(f, "member joined")
            }
            MessageType::Call(call_msg) => {
                if call_msg.ended_at.is_some() {
                    write!(f, "call (ended)")
                } else {
                    write!(f, "call (active)")
                }
            }
            MessageType::ChannelRename(rename) => {
                write!(f, "channel renamed from to \"{}\"", rename.name_new)
            }
            MessageType::ChannelPingback(_) => {
                write!(f, "channel pingback")
            }
            MessageType::ChannelMoved(_) => {
                write!(f, "channel moved")
            }
            MessageType::ChannelTagged(_) => {
                write!(f, "channel tagged")
            }
            MessageType::ChannelIcon(_) => {
                write!(f, "channel icon changed")
            }
            MessageType::ThreadCreated(thread_msg) => {
                if thread_msg.source_message_id.is_some() {
                    write!(f, "thread created from message")
                } else {
                    write!(f, "thread created")
                }
            }
            MessageType::AutomodExecution(_exec) => {
                write!(f, "auto moderation action executed")

                // TODO: log rule names, matches, etc
                // write!(
                //     f,
                //     "Auto moderation action executed",
                //     automod_msg.matches[0].matches,
                //     automod_msg.rules.iter().map(|r| r.name.as_str()).join(" ")
                // )
            }
            #[cfg(feature = "feat_message_move")]
            MessageType::MessagesMoved(move_msg) => {
                write!(f, "messages moved ",)
            }

            MessageType::DocumentTag(_) => {
                write!(f, "document tag created")
            }
            MessageType::DocumentEdits(_) => {
                write!(f, "document edit summary")
            }
            MessageType::DocumentMerged(_) => {
                write!(f, "document branch merged")
            }
            MessageType::Nudge => {
                write!(f, "nudge")
            }
        }
    }
}
