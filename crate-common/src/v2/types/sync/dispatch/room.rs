use lamprey_macros::record;

use super::invite::{InviteCreate, InviteDelete, InviteDeleteBulk, InviteUpdate};
use crate::v1::types::{
    automod::{AutomodRule, AutomodRuleExecution},
    emoji::EmojiCustom,
    mirror::RoomSeq,
    AuditLogEntry, AutomodRuleId, Channel, ChannelReorderItem, EmojiId, Role, RoleId,
    RoleReorderItem, Room, RoomBan, RoomId, RoomMember, User, UserId,
};

/// something happened in a room
///
/// requires a subscription to the room to receive
#[record]
#[serde(tag = "type")]
pub enum DispatchRoom {
    RoomCreate(RoomCreate),
    RoomUpdate(RoomUpdate),
    RoomDelete(RoomDelete),
    RoleCreate(RoleCreate),
    RoleUpdate(RoleUpdate),
    RoleDelete(RoleDelete),
    RoleReorder(RoleReorder),
    RoomMemberCreate(RoomMemberCreate),
    RoomMemberUpdate(RoomMemberUpdate),
    RoomMemberDelete(RoomMemberDelete),
    ChannelReorder(ChannelReorder),
    EmojiCreate(EmojiCreate),
    EmojiUpdate(EmojiUpdate),
    EmojiDelete(EmojiDelete),
    BanCreate(BanCreate),
    BanDelete(BanDelete),
    AuditLogEntryCreate(AuditLogEntryCreate),
    AutomodRuleCreate(AutomodRuleCreate),
    AutomodRuleUpdate(AutomodRuleUpdate),
    AutomodRuleDelete(AutomodRuleDelete),
    AutomodRuleExecute(AutomodRuleExecute),
    InviteCreate(RoomInviteCreate),
    InviteUpdate(RoomInviteUpdate),
    InviteDelete(RoomInviteDelete),
    InviteDeleteBulk(RoomInviteDeleteBulk),
}

/// a room was created and/or you joined a room
#[record]
pub struct RoomCreate {
    pub room: Box<Room>,
    pub roles: Vec<Role>,
    pub channels: Vec<Channel>,
    pub threads: Vec<Channel>,

    /// your own room member
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_member: Option<Box<RoomMember>>,

    /// the room sync sequence number of this event, for offline sync
    // TODO: this should probably be a part of room
    pub seq: RoomSeq,
}

/// a room was updated
#[record]
pub struct RoomUpdate {
    pub room: Box<Room>,
    pub seq: Option<RoomSeq>,
}

/// a room was deleted, you left a room, or you were removed (kicked/banned) from a room
#[record]
pub struct RoomDelete {
    pub room_id: RoomId,
    pub seq: RoomSeq,
}

/// a role was created
#[record]
pub struct RoleCreate {
    pub role: Box<Role>,
    pub seq: RoomSeq,
}

/// a role was updated
#[record]
pub struct RoleUpdate {
    pub role: Box<Role>,
    pub seq: RoomSeq,
}

/// a role was deleted
#[record]
pub struct RoleDelete {
    pub room_id: RoomId,
    pub role_id: RoleId,
    pub seq: RoomSeq,
}

/// the role hierarchy was reordered
#[record]
pub struct RoleReorder {
    pub room_id: RoomId,
    pub roles: Vec<RoleReorderItem>,
    pub seq: RoomSeq,
}

/// a room member was created (join)
#[record]
pub struct RoomMemberCreate {
    pub member: Box<RoomMember>,
    pub user: Box<User>,
    pub seq: RoomSeq,
}

/// a room member was updated
#[record]
pub struct RoomMemberUpdate {
    pub member: Box<RoomMember>,
    pub user: Box<User>,
    pub seq: RoomSeq,
}

/// a room member was deleted (leave/kick/ban)
#[record]
pub struct RoomMemberDelete {
    pub room_id: RoomId,
    pub user_id: UserId,
    pub seq: RoomSeq,
}

/// the channels in a room were reordered
#[record]
pub struct ChannelReorder {
    pub room_id: RoomId,
    pub channels: Vec<ChannelReorderItem>,
    pub seq: RoomSeq,
}

/// a custom emoji was created
#[record]
pub struct EmojiCreate {
    pub room_id: RoomId,
    pub emoji: Box<EmojiCustom>,
    pub seq: RoomSeq,
}

/// a custom emoji was updated
#[record]
pub struct EmojiUpdate {
    pub room_id: RoomId,
    pub emoji: Box<EmojiCustom>,
    pub seq: RoomSeq,
}

/// a custom emoji was deleted
#[record]
pub struct EmojiDelete {
    pub emoji_id: EmojiId,
    pub room_id: RoomId,
    pub seq: RoomSeq,
}

/// a user was banned from a room
#[record]
pub struct BanCreate {
    pub room_id: RoomId,
    pub ban: Box<RoomBan>,
    pub seq: RoomSeq,
}

/// a user was unbanned from a room
#[record]
pub struct BanDelete {
    pub room_id: RoomId,
    pub user_id: UserId,
    pub seq: RoomSeq,
}

/// an audit log entry was created
#[record]
pub struct AuditLogEntryCreate {
    pub entry: Box<AuditLogEntry>,
    pub seq: RoomSeq,
}

/// an automod rule was created
#[record]
pub struct AutomodRuleCreate {
    pub rule: Box<AutomodRule>,
    pub seq: RoomSeq,
}

/// an automod rule was updated
#[record]
pub struct AutomodRuleUpdate {
    pub rule: Box<AutomodRule>,
    pub seq: RoomSeq,
}

/// an automod rule was deleted
#[record]
pub struct AutomodRuleDelete {
    pub rule_id: AutomodRuleId,
    pub room_id: RoomId,
    pub seq: RoomSeq,
}

/// an automod rule was executed
#[record]
pub struct AutomodRuleExecute {
    pub execution: Box<AutomodRuleExecution>,
    pub seq: RoomSeq,
}

/// an invite in this room was created
#[record]
pub struct RoomInviteCreate {
    pub room_id: RoomId,
    #[serde(flatten)]
    pub invite: InviteCreate,
    pub seq: RoomSeq,
}

/// an invite in this room was updated
#[record]
pub struct RoomInviteUpdate {
    pub room_id: RoomId,
    #[serde(flatten)]
    pub invite: InviteUpdate,
    pub seq: RoomSeq,
}

/// an invite in this room was deleted
#[record]
pub struct RoomInviteDelete {
    pub room_id: RoomId,
    #[serde(flatten)]
    pub invite: InviteDelete,
    pub seq: RoomSeq,
}

/// multiple invites in this room were deleted
#[record]
pub struct RoomInviteDeleteBulk {
    pub room_id: RoomId,
    #[serde(flatten)]
    pub invite: InviteDeleteBulk,
    pub seq: RoomSeq,
}

impl DispatchRoom {
    /// id of the room this event happened in
    pub fn room_id(&self) -> RoomId {
        match self {
            DispatchRoom::RoomCreate(a) => a.room.id,
            DispatchRoom::RoomUpdate(a) => a.room.id,
            DispatchRoom::RoomDelete(a) => a.room_id,
            DispatchRoom::RoleCreate(a) => a.role.room_id,
            DispatchRoom::RoleUpdate(a) => a.role.room_id,
            DispatchRoom::RoleDelete(a) => a.room_id,
            DispatchRoom::RoleReorder(a) => a.room_id,
            DispatchRoom::RoomMemberCreate(a) => a.member.room_id,
            DispatchRoom::RoomMemberUpdate(a) => a.member.room_id,
            DispatchRoom::RoomMemberDelete(a) => a.room_id,
            DispatchRoom::ChannelReorder(a) => a.room_id,
            DispatchRoom::EmojiCreate(a) => a.room_id,
            DispatchRoom::EmojiUpdate(a) => a.room_id,
            DispatchRoom::EmojiDelete(a) => a.room_id,
            DispatchRoom::BanCreate(a) => a.room_id,
            DispatchRoom::BanDelete(a) => a.room_id,
            DispatchRoom::AuditLogEntryCreate(a) => a.entry.room_id,
            DispatchRoom::AutomodRuleCreate(a) => a.rule.room_id,
            DispatchRoom::AutomodRuleUpdate(a) => a.rule.room_id,
            DispatchRoom::AutomodRuleDelete(a) => a.room_id,
            DispatchRoom::AutomodRuleExecute(a) => a.execution.room_id,
            DispatchRoom::InviteCreate(a) => a.room_id,
            DispatchRoom::InviteUpdate(a) => a.room_id,
            DispatchRoom::InviteDelete(a) => a.room_id,
            DispatchRoom::InviteDeleteBulk(a) => a.room_id,
        }
    }

    /// the room sync sequence number of this event
    ///
    /// if None, this event doesnt increment the seq. used for offline sync.
    pub fn seq(&self) -> Option<RoomSeq> {
        match self {
            DispatchRoom::RoomCreate(a) => Some(a.seq),
            DispatchRoom::RoomUpdate(a) => a.seq,
            DispatchRoom::RoomDelete(a) => Some(a.seq),
            DispatchRoom::RoleCreate(a) => Some(a.seq),
            DispatchRoom::RoleUpdate(a) => Some(a.seq),
            DispatchRoom::RoleDelete(a) => Some(a.seq),
            DispatchRoom::RoleReorder(a) => Some(a.seq),
            DispatchRoom::RoomMemberCreate(a) => Some(a.seq),
            DispatchRoom::RoomMemberUpdate(a) => Some(a.seq),
            DispatchRoom::RoomMemberDelete(a) => Some(a.seq),
            DispatchRoom::ChannelReorder(a) => Some(a.seq),
            DispatchRoom::EmojiCreate(a) => Some(a.seq),
            DispatchRoom::EmojiUpdate(a) => Some(a.seq),
            DispatchRoom::EmojiDelete(a) => Some(a.seq),
            DispatchRoom::BanCreate(a) => Some(a.seq),
            DispatchRoom::BanDelete(a) => Some(a.seq),
            DispatchRoom::AuditLogEntryCreate(a) => Some(a.seq),
            DispatchRoom::AutomodRuleCreate(a) => Some(a.seq),
            DispatchRoom::AutomodRuleUpdate(a) => Some(a.seq),
            DispatchRoom::AutomodRuleDelete(a) => Some(a.seq),
            DispatchRoom::AutomodRuleExecute(a) => Some(a.seq),
            DispatchRoom::InviteCreate(a) => Some(a.seq),
            DispatchRoom::InviteUpdate(a) => Some(a.seq),
            DispatchRoom::InviteDelete(a) => Some(a.seq),
            DispatchRoom::InviteDeleteBulk(a) => Some(a.seq),
        }
    }
}
