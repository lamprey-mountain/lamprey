use common::{
    v1::types::{InviteTarget, InviteTargetId, MessageSync, emoji::EmojiOwner},
    v2::types::{ChannelId, RoomId},
};

// TODO: move to common
/// get the room id for a message sync event
pub fn sync_room_id(sync: &MessageSync) -> Option<RoomId> {
    match sync {
        MessageSync::RoomCreate { room } => Some(room.id),
        MessageSync::RoomUpdate { room } => Some(room.id),
        MessageSync::RoomDelete { room_id } => Some(*room_id),
        MessageSync::ChannelCreate { channel } => channel.room_id,
        MessageSync::ChannelUpdate { channel } => channel.room_id,
        MessageSync::RoomMemberCreate { member, .. } => Some(member.room_id),
        MessageSync::RoomMemberUpdate { member, .. } => Some(member.room_id),
        MessageSync::RoomMemberDelete { room_id, .. } => Some(*room_id),
        MessageSync::RoleCreate { role } => Some(role.room_id),
        MessageSync::RoleUpdate { role } => Some(role.room_id),
        MessageSync::RoleDelete { room_id, .. } => Some(*room_id),
        MessageSync::RoleReorder { room_id, .. } => Some(*room_id),
        MessageSync::InviteCreate { invite } => match &invite.invite.target {
            InviteTarget::Room { room, .. } => Some(room.id),
            _ => None,
        },
        MessageSync::InviteUpdate { invite } => match &invite.invite.target {
            InviteTarget::Room { room, .. } => Some(room.id),
            _ => None,
        },
        MessageSync::InviteDelete { target, .. } => match target {
            InviteTargetId::Room { room_id, .. } => Some(*room_id),
            _ => None,
        },
        MessageSync::EmojiCreate { emoji } => match &emoji.owner {
            Some(EmojiOwner::Room { room_id }) => Some(*room_id),
            _ => None,
        },
        MessageSync::EmojiUpdate { emoji } => match &emoji.owner {
            Some(EmojiOwner::Room { room_id }) => Some(*room_id),
            _ => None,
        },
        MessageSync::EmojiDelete { room_id, .. } => Some(*room_id),
        MessageSync::AuditLogEntryCreate { entry } => Some(entry.room_id),
        MessageSync::AutomodRuleCreate { rule } => Some(rule.room_id),
        MessageSync::AutomodRuleUpdate { rule } => Some(rule.room_id),
        MessageSync::AutomodRuleDelete { room_id, .. } => Some(*room_id),
        MessageSync::WebhookCreate { webhook } => webhook.room_id,
        MessageSync::WebhookUpdate { webhook } => webhook.room_id,
        MessageSync::WebhookDelete { room_id, .. } => *room_id,

        _ => None,
    }
}

// TODO: move to common
pub fn sync_channel_id(sync: &MessageSync) -> Option<ChannelId> {
    match sync {
        MessageSync::ChannelCreate { channel } => Some(channel.id),
        // TODO: handle more events
        _ => None,
    }
}
