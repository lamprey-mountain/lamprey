use super::Permission;

/// Which permissions are granted to someone with Admin in a room
pub const ADMIN_ROOM: &[Permission] = &[
    Permission::Admin,
    Permission::IntegrationsManage,
    Permission::IntegrationsManage,
    Permission::EmojiManage,
    Permission::EmojiUseExternal,
    Permission::InviteCreate,
    Permission::InviteManage,
    Permission::MemberBan,
    Permission::MemberBridge,
    Permission::MemberKick,
    Permission::MemberNicknameManage,
    Permission::MemberTimeout,
    Permission::MessageCreate,
    Permission::MessageRemove,
    Permission::MessageDelete,
    Permission::MessageEmbeds,
    Permission::MessageMassMention,
    Permission::MessageAttachments,
    Permission::MessageMove,
    Permission::MessagePin,
    Permission::MemberNickname,
    Permission::ReactionAdd,
    Permission::ReactionPurge,
    Permission::RoleApply,
    Permission::RoleManage,
    Permission::RoomManage,
    Permission::ServerMetrics,
    Permission::ServerOversee,
    Permission::ServerReports,
    Permission::TagApply,
    Permission::TagManage,
    Permission::ThreadCreatePublic,
    Permission::ThreadCreatePrivate,
    Permission::ThreadManage,
    Permission::ThreadEdit,
    Permission::ThreadLock,
    Permission::ThreadManage,
    Permission::ViewChannel,
    Permission::ViewAuditLog,
    Permission::VoiceConnect,
    Permission::VoiceDeafen,
    Permission::VoiceDisconnect,
    Permission::VoiceMove,
    Permission::VoiceMute,
    Permission::VoicePriority,
    Permission::VoiceSpeak,
    Permission::VoiceVideo,
    Permission::ChannelManage,
    Permission::ChannelEdit,
];

/// Which permissions are granted to someone with Admin in a thread
///
/// Some of these might be unintentionally re-added (eg. EmojiUseExternal), but
/// thats what Admin is supposed to do
pub const ADMIN_THREAD: &[Permission] = &[
    Permission::Admin,
    Permission::IntegrationsManage,
    Permission::IntegrationsManage,
    Permission::EmojiUseExternal,
    Permission::InviteCreate,
    Permission::InviteManage,
    Permission::MemberBan,
    Permission::MemberBridge,
    Permission::MemberKick,
    Permission::MemberNicknameManage,
    Permission::MemberTimeout,
    Permission::MessageCreate,
    Permission::MessageRemove,
    Permission::MessageDelete,
    Permission::MessageEmbeds,
    Permission::MessageMassMention,
    Permission::MessageAttachments,
    Permission::MessageMove,
    Permission::MessagePin,
    Permission::ReactionAdd,
    Permission::MemberNickname,
    Permission::ReactionAdd,
    Permission::ReactionPurge,
    Permission::TagApply,
    Permission::ThreadManage,
    Permission::ThreadEdit,
    Permission::ThreadLock,
    Permission::ThreadManage,
    Permission::ViewChannel,
    Permission::VoiceConnect,
    Permission::VoiceDeafen,
    Permission::VoiceDisconnect,
    Permission::VoiceMove,
    Permission::VoiceMute,
    Permission::VoicePriority,
    Permission::VoiceSpeak,
    Permission::VoiceVideo,
    Permission::ChannelEdit,
];

/// Default permissions for everyone in a trusted room (eg. with friends)
pub const EVERYONE_TRUSTED: &[Permission] = &[
    Permission::IntegrationsManage,
    Permission::EmojiUseExternal,
    Permission::InviteCreate,
    Permission::MessageCreate,
    Permission::MessageEmbeds,
    Permission::MessageMassMention, // maybe?
    Permission::MessageAttachments,
    Permission::MessageMove, // maybe?
    Permission::MessagePin,  // maybe?
    Permission::ReactionAdd,
    Permission::MemberNickname,
    Permission::TagApply,
    // Permission::TagManage, // maybe?
    Permission::ThreadCreatePublic,
    Permission::ThreadCreatePrivate,
    Permission::ThreadEdit, // maybe?
    Permission::ViewChannel,
    // Permission::ThreadPin, // maybe?
    // Permission::ThreadPublish, // maybe?
    Permission::ViewAuditLog, // maybe?
    Permission::VoiceConnect,
    Permission::VoiceSpeak,
    Permission::VoiceVideo,
    Permission::ChannelEdit,
];

/// Default permissions for everyone in an untrusted room (eg. public)
pub const EVERYONE_UNTRUSTED: &[Permission] = &[
    Permission::EmojiUseExternal,
    Permission::InviteCreate,
    Permission::MessageCreate,
    Permission::MessageEmbeds,
    Permission::MessageAttachments,
    Permission::ReactionAdd,
    Permission::MemberNickname,
    Permission::TagApply, // maybe?
    Permission::ThreadCreatePublic,
    Permission::ThreadCreatePrivate,
    Permission::ThreadEdit, // maybe?
    Permission::ViewChannel,
    // Permission::ViewAuditLog, // maybe?
    Permission::VoiceConnect,
    Permission::VoiceSpeak,
    Permission::VoiceVideo,
];

/// extra permissions for someone who moderates stuff
pub const MODERATOR: &[Permission] = &[
    Permission::InviteManage,
    Permission::MemberBan,
    Permission::MemberKick,
    Permission::MemberNicknameManage,
    Permission::MemberTimeout,
    Permission::MessageDelete,
    Permission::MessageMove,
    Permission::MessagePin, // maybe?
    Permission::MessageRemove,
    Permission::ReactionPurge,
    Permission::RoleApply, // maybe?
    Permission::TagApply,
    // Permission::TagManage, // maybe?
    Permission::ThreadManage,
    Permission::ThreadEdit,
    Permission::ThreadLock,
    Permission::ThreadManage, // maybe?
    Permission::ViewChannel,
    Permission::ViewAuditLog, // maybe?
    Permission::VoiceDeafen,
    Permission::VoiceDisconnect,
    Permission::VoiceMove,
    Permission::VoiceMute,
    Permission::VoicePriority, // maybe?
];
