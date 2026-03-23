//! authorization checks

use common::{
    v1::types::{
        emoji::EmojiOwner, ChannelId, ConnectionId, InviteTarget, InviteTargetId, MessageSync,
        Permission, RoomId, SessionId, UserId,
    },
    v2::types::media::MediaLinkType,
};

/// the auth checks that must pass in order to view this event
#[derive(Debug)]
pub enum AuthCheck {
    /// must be able to view this room
    ///
    /// allows lurkers
    Room(RoomId),

    /// must have this permission in this room
    ///
    /// allows lurkers
    RoomPerm(RoomId, Permission),

    /// must be able to view this channel
    ///
    /// allows lurkers
    Channel(ChannelId),

    /// must have this permission in this channel
    ChannelPerm(ChannelId, Permission),

    /// must be this user
    User(UserId),

    /// must be able to see this user
    ///
    /// - friends
    /// - mutual rooms
    /// - mutual gdms
    UserVisible(UserId),

    /// must be this session
    Session(SessionId),

    /// must be this connection
    Connection(ConnectionId),

    /// any of these checks must pass
    Any(Vec<AuthCheck>),
}

impl AuthCheck {
    /// return an auth check for "either in this room or is this user"
    fn room_or_user(room_id: RoomId, user_id: UserId) -> Self {
        AuthCheck::Any(vec![AuthCheck::Room(room_id), AuthCheck::User(user_id)])
    }

    /// calculate the required auth checks for this message
    pub fn for_message(msg: &MessageSync) -> Self {
        match msg {
            MessageSync::Ambient { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::RoomCreate { room } => AuthCheck::Room(room.id),
            MessageSync::RoomUpdate { room } => AuthCheck::Room(room.id),
            MessageSync::RoomDelete { room_id } => AuthCheck::Room(*room_id),
            MessageSync::ChannelCreate { channel } => AuthCheck::Channel(channel.id),
            MessageSync::ChannelUpdate { channel } => AuthCheck::Channel(channel.id),
            MessageSync::MessageCreate { message } => AuthCheck::Channel(message.channel_id),
            MessageSync::MessageUpdate { message } => AuthCheck::Channel(message.channel_id),
            MessageSync::UserCreate { user } => AuthCheck::UserVisible(user.id),
            MessageSync::UserUpdate { user } => AuthCheck::UserVisible(user.id),
            MessageSync::PresenceUpdate { user_id, .. } => AuthCheck::UserVisible(*user_id),
            MessageSync::PreferencesGlobal { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::PreferencesRoom { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::PreferencesChannel { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::PreferencesUser { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::RoomMemberCreate { member, .. } => {
                AuthCheck::room_or_user(member.room_id, member.user_id)
            }

            MessageSync::RoomMemberUpdate { member, .. } => {
                AuthCheck::room_or_user(member.room_id, member.user_id)
            }
            MessageSync::RoomMemberDelete { room_id, user_id } => {
                AuthCheck::room_or_user(*room_id, *user_id)
            }
            MessageSync::ThreadMemberUpsert { thread_id, .. } => AuthCheck::Channel(*thread_id),
            MessageSync::SessionCreate { session } | MessageSync::SessionUpdate { session } => {
                // copied from session.can_see
                match session.user_id() {
                    Some(user_id) => AuthCheck::Any(vec![
                        AuthCheck::Session(session.id),
                        AuthCheck::User(user_id),
                    ]),
                    None => AuthCheck::Session(session.id),
                }
            }
            MessageSync::RoleCreate { role } => AuthCheck::Room(role.room_id),
            MessageSync::RoleUpdate { role } => AuthCheck::Room(role.room_id),
            MessageSync::InviteCreate { invite } | MessageSync::InviteUpdate { invite } => {
                let mut checks = vec![AuthCheck::User(invite.invite.creator_id)];
                match &invite.invite.target {
                    InviteTarget::Room { room, channel, .. } => {
                        checks.push(AuthCheck::RoomPerm(room.id, Permission::InviteManage));
                        if let Some(channel) = channel {
                            checks
                                .push(AuthCheck::ChannelPerm(channel.id, Permission::InviteManage));
                        }
                    }
                    InviteTarget::Gdm { channel } => {
                        checks.push(AuthCheck::ChannelPerm(channel.id, Permission::InviteManage));
                    }
                    InviteTarget::Server => {
                        checks.push(AuthCheck::RoomPerm(
                            common::v1::types::SERVER_ROOM_ID,
                            Permission::InviteManage,
                        ));
                    }
                    InviteTarget::User { user } => {
                        checks.push(AuthCheck::User(user.id));
                    }
                }
                AuthCheck::Any(checks)
            }
            MessageSync::MessageDelete { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::MessageVersionDelete { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::UserDelete { id } => AuthCheck::UserVisible(*id),
            MessageSync::SessionDelete { id, user_id } => {
                if let Some(user_id) = user_id {
                    AuthCheck::Any(vec![AuthCheck::Session(*id), AuthCheck::User(*user_id)])
                } else {
                    AuthCheck::Session(*id)
                }
            }
            MessageSync::SessionDeleteAll { user_id } => AuthCheck::User(*user_id),
            MessageSync::RoleDelete { room_id, .. } => AuthCheck::Room(*room_id),
            MessageSync::RoleReorder { room_id, .. } => AuthCheck::Room(*room_id),
            MessageSync::InviteDelete {
                target, creator_id, ..
            } => {
                let mut checks = vec![AuthCheck::User(*creator_id)];
                match target {
                    InviteTargetId::Room {
                        room_id,
                        channel_id,
                        ..
                    } => {
                        checks.push(AuthCheck::RoomPerm(*room_id, Permission::InviteManage));
                        if let Some(channel_id) = channel_id {
                            checks.push(AuthCheck::ChannelPerm(
                                *channel_id,
                                Permission::InviteManage,
                            ));
                        }
                    }
                    InviteTargetId::Gdm { channel_id } => {
                        checks.push(AuthCheck::ChannelPerm(
                            *channel_id,
                            Permission::InviteManage,
                        ));
                    }
                    InviteTargetId::Server => {
                        checks.push(AuthCheck::RoomPerm(
                            common::v1::types::SERVER_ROOM_ID,
                            Permission::InviteManage,
                        ));
                    }
                    InviteTargetId::User { user_id } => {
                        checks.push(AuthCheck::User(*user_id));
                    }
                }
                AuthCheck::Any(checks)
            }
            MessageSync::ChannelTyping { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::ChannelAck { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::RelationshipUpsert { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::RelationshipDelete { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::ReactionCreate { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::ReactionDelete { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::ReactionDeleteKey { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::ReactionDeleteAll { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::MessageDeleteBulk { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::MessageRemove { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::MessageRestore { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::VoiceDispatch { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::VoiceState {
                state,
                user_id,
                old_state,
            } => match (state, old_state) {
                (None, None) => AuthCheck::User(*user_id),
                (None, Some(o)) => AuthCheck::Channel(o.channel_id),
                (Some(s), None) => AuthCheck::Channel(s.channel_id),
                (Some(s), Some(o)) => AuthCheck::Any(vec![
                    AuthCheck::Channel(s.channel_id),
                    AuthCheck::Channel(o.channel_id),
                ]),
            },
            MessageSync::CallCreate { call } => AuthCheck::Channel(call.channel_id),
            MessageSync::CallUpdate { call } => AuthCheck::Channel(call.channel_id),
            MessageSync::CallDelete { channel_id } => AuthCheck::Channel(*channel_id),
            MessageSync::EmojiCreate { emoji } => match emoji
                .owner
                .as_ref()
                .expect("emoji sync events from server always has owner")
            {
                EmojiOwner::Room { room_id } => AuthCheck::Room(*room_id),
                EmojiOwner::User => AuthCheck::User(
                    emoji
                        .creator_id
                        .expect("emoji sync events from server always has creator_id"),
                ),
            },
            MessageSync::EmojiUpdate { emoji } => match emoji
                .owner
                .as_ref()
                .expect("emoji sync events from server always has owner")
            {
                EmojiOwner::Room { room_id } => AuthCheck::Room(*room_id),
                EmojiOwner::User => AuthCheck::User(
                    emoji
                        .creator_id
                        .expect("emoji sync events from server always has creator_id"),
                ),
            },
            MessageSync::EmojiDelete {
                room_id,
                emoji_id: _,
            } => AuthCheck::Room(*room_id),
            MessageSync::ConnectionCreate { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::ConnectionDelete { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::AuditLogEntryCreate { entry } => {
                AuthCheck::RoomPerm(entry.room_id, Permission::AuditLogView)
            }
            MessageSync::BanCreate { room_id, .. } => {
                AuthCheck::RoomPerm(*room_id, Permission::MemberBan)
            }
            MessageSync::BanDelete { room_id, .. } => {
                AuthCheck::RoomPerm(*room_id, Permission::MemberBan)
            }
            MessageSync::AutomodRuleCreate { rule } => {
                AuthCheck::RoomPerm(rule.room_id, Permission::RoomEdit)
            }
            MessageSync::AutomodRuleUpdate { rule } => {
                AuthCheck::RoomPerm(rule.room_id, Permission::RoomEdit)
            }
            MessageSync::AutomodRuleDelete { room_id, .. } => {
                AuthCheck::RoomPerm(*room_id, Permission::RoomEdit)
            }
            MessageSync::AutomodRuleExecute { execution } => {
                AuthCheck::RoomPerm(execution.rule.room_id, Permission::RoomEdit)
            }
            MessageSync::MemberListSync { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::InboxNotificationCreate { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::InboxMarkRead { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::InboxMarkUnread { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::InboxFlush { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::CalendarEventCreate { event } => AuthCheck::Channel(event.channel_id),
            MessageSync::CalendarEventUpdate { event } => AuthCheck::Channel(event.channel_id),
            MessageSync::CalendarEventDelete { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::CalendarOverwriteCreate { channel_id, .. } => {
                AuthCheck::Channel(*channel_id)
            }
            MessageSync::CalendarOverwriteUpdate { channel_id, .. } => {
                AuthCheck::Channel(*channel_id)
            }
            MessageSync::CalendarOverwriteDelete { channel_id, .. } => {
                AuthCheck::Channel(*channel_id)
            }
            MessageSync::CalendarRsvpCreate { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::CalendarRsvpDelete { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::CalendarOverwriteRsvpCreate { channel_id, .. } => {
                AuthCheck::Channel(*channel_id)
            }
            MessageSync::CalendarOverwriteRsvpDelete { channel_id, .. } => {
                AuthCheck::Channel(*channel_id)
            }
            MessageSync::WebhookCreate { webhook } => {
                AuthCheck::ChannelPerm(webhook.channel_id, Permission::IntegrationsManage)
            }
            MessageSync::WebhookUpdate { webhook } => {
                AuthCheck::ChannelPerm(webhook.channel_id, Permission::IntegrationsManage)
            }
            MessageSync::WebhookDelete { channel_id, .. } => {
                AuthCheck::ChannelPerm(*channel_id, Permission::IntegrationsManage)
            }
            MessageSync::RatelimitUpdate { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::HarvestUpdate { harvest, .. } => AuthCheck::User(harvest.user_id),
            MessageSync::DocumentEdit { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::DocumentPresence { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::DocumentSubscribed { connection_id, .. } => {
                AuthCheck::Connection(*connection_id)
            }
            MessageSync::DocumentTagCreate { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::DocumentTagUpdate { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::DocumentTagDelete { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::DocumentBranchCreate { branch } => AuthCheck::Channel(branch.document_id),
            MessageSync::DocumentBranchUpdate { branch } => AuthCheck::Channel(branch.document_id),
            MessageSync::DocumentBranchDelete { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::TagCreate { tag } => AuthCheck::Channel(tag.channel_id),
            MessageSync::TagUpdate { tag } => AuthCheck::Channel(tag.channel_id),
            MessageSync::TagDelete { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::MediaProcessed { session_id, .. } => AuthCheck::Session(*session_id),
            MessageSync::MediaUpdate { media } => {
                if media.links.is_empty() {
                    AuthCheck::User(media.user_id.expect("server always has media.user_id"))
                } else {
                    let mut auth_checks = Vec::new();
                    auth_checks.push(AuthCheck::User(
                        media.user_id.expect("server always has media.user_id"),
                    ));

                    for link in &media.links {
                        let check = match link {
                            MediaLinkType::Message { channel_id, .. } => {
                                AuthCheck::Channel(*channel_id)
                            }
                            MediaLinkType::MessageVersion { channel_id, .. } => {
                                AuthCheck::Channel(*channel_id)
                            }
                            MediaLinkType::UserAvatar { user_id } => AuthCheck::User(*user_id),
                            MediaLinkType::UserBanner { user_id } => AuthCheck::User(*user_id),
                            MediaLinkType::ChannelIcon { channel_id } => {
                                AuthCheck::Channel(*channel_id)
                            }
                            MediaLinkType::RoomIcon { room_id } => AuthCheck::Room(*room_id),
                            MediaLinkType::Embed { id: _ } => {
                                // Embeds are linked to messages, check channel perms for embed message
                                // For now, fall back to user who uploaded
                                continue;
                            }
                            MediaLinkType::CustomEmoji { room_id } => AuthCheck::Room(*room_id),
                            MediaLinkType::RoomBanner { room_id } => AuthCheck::Room(*room_id),
                        };
                        auth_checks.push(check);
                    }

                    AuthCheck::Any(auth_checks)
                }
            }
        }
    }
}
