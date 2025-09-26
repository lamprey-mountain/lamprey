use super::Permission;

/// Which permissions are granted to someone with Admin in a room
pub const ADMIN_ROOM: &[Permission] = &[
    Permission::Admin,
    Permission::IntegrationsManage,
    Permission::IntegrationsManage,
    Permission::UnusedEmojiAdd,
    Permission::EmojiManage,
    Permission::EmojiUseExternal,
    Permission::InviteCreate,
    Permission::InviteManage,
    Permission::MemberBan,
    Permission::UnusedMemberBanManage,
    Permission::MemberBridge,
    Permission::MemberKick,
    Permission::MemberNicknameManage,
    Permission::MessageCreate,
    Permission::MessageDelete,
    // Permission::MessageEdit, // internal
    Permission::MessageEmbeds,
    Permission::MessageMassMention,
    Permission::MessageAttachments,
    Permission::MessageMove,
    Permission::MessagePin,
    Permission::UnusedProfileAvatar,
    Permission::MemberNickname,
    Permission::ReactionAdd,
    Permission::ReactionPurge,
    Permission::RoleApply,
    Permission::RoleManage,
    Permission::RoomManage,
    Permission::UnusedServerAdmin,
    Permission::ServerMetrics,
    Permission::ServerOversee,
    Permission::ServerReports,
    Permission::TagApply,
    Permission::TagManage,
    Permission::ThreadArchive,
    Permission::ThreadCreateChat,
    Permission::UnusedThreadCreateDocument,
    Permission::UnusedThreadCreateEvent,
    Permission::ThreadCreateForum,
    Permission::UnusedThreadCreateForum2,
    Permission::UnusedThreadCreateTable,
    Permission::ThreadCreateVoice,
    Permission::ThreadCreatePublic,
    Permission::ThreadCreatePrivate,
    Permission::ThreadRemove,
    Permission::ThreadEdit,
    Permission::ThreadForward,
    Permission::ThreadLock,
    Permission::ThreadManage,
    Permission::ThreadPublish,
    // Permission::UserDms,      // user perm doesnt apply
    // Permission::UserProfile,  // user perm doesnt apply
    // Permission::UserSessions, // user perm doesnt apply
    // Permission::UserStatus,   // user perm doesnt apply
    // Permission::View, // internal
    Permission::ViewAuditLog,
    Permission::VoiceConnect,
    Permission::VoiceDeafen,
    Permission::VoiceDisconnect,
    Permission::VoiceMove,
    Permission::VoiceMute,
    Permission::VoicePriority,
    Permission::VoiceSpeak,
    Permission::VoiceVideo,
];

/// Which permissions are granted to someone with Admin in a thread
///
/// Some of these might be unintentionally re-added (eg. EmojiUseExternal), but
/// thats what Admin is supposed to do
pub const ADMIN_THREAD: &[Permission] = &[
    Permission::Admin,
    Permission::IntegrationsManage,
    Permission::IntegrationsManage,
    // Permission::EmojiAdd,    // room perm doesnt apply
    // Permission::EmojiManage, // room perm doesnt apply
    Permission::EmojiUseExternal,
    Permission::InviteCreate,
    Permission::InviteManage,
    Permission::MemberBan,
    Permission::UnusedMemberBanManage,
    Permission::MemberBridge,
    Permission::MemberKick,
    Permission::MemberNicknameManage,
    Permission::MessageCreate,
    Permission::MessageDelete,
    // Permission::MessageEdit, // internal
    Permission::MessageEmbeds,
    Permission::MessageMassMention,
    Permission::MessageAttachments,
    Permission::MessageMove,
    Permission::MessagePin,
    Permission::ReactionAdd,
    Permission::UnusedProfileAvatar,
    Permission::MemberNickname,
    Permission::ReactionAdd,
    Permission::ReactionPurge,
    // Permission::RoleApply,  // room perm doesnt apply
    // Permission::RoleManage, // room perm doesnt apply
    // Permission::RoomEdit,   // room perm doesnt apply
    // Permission::ServerAdmin,   // server perm doesnt apply
    // Permission::ServerMetrics, // server perm doesnt apply
    // Permission::ServerOversee, // server perm doesnt apply
    // Permission::ServerReports, // server perm doesnt apply
    Permission::TagApply,
    // Permission::TagManage, // room perm doesnt apply
    Permission::ThreadArchive,
    // Permission::ThreadCreateChat,        // room perm doesnt apply
    // Permission::ThreadCreateDocument,    // room perm doesnt apply
    // Permission::ThreadCreateEvent,       // room perm doesnt apply
    // Permission::ThreadCreateForumLinear, // room perm doesnt apply
    // Permission::ThreadCreateForumTree,   // room perm doesnt apply
    // Permission::ThreadCreateTable,       // room perm doesnt apply
    // Permission::ThreadCreateVoice,       // room perm doesnt apply
    // Permission::ThreadCreatePublic,      // room perm doesnt apply
    // Permission::ThreadCreatePrivate,     // room perm doesnt apply
    Permission::ThreadRemove,
    Permission::ThreadEdit,
    Permission::ThreadForward,
    Permission::ThreadLock,
    Permission::ThreadManage,
    Permission::ThreadPublish,
    // Permission::UserDms,      // user perm doesnt apply
    // Permission::UserProfile,  // user perm doesnt apply
    // Permission::UserSessions, // user perm doesnt apply
    // Permission::UserStatus,   // user perm doesnt apply
    // Permission::View, // internal
    // Permission::ViewAuditLog, // room perm doesnt apply
    Permission::VoiceConnect,
    Permission::VoiceDeafen,
    Permission::VoiceDisconnect,
    Permission::VoiceMove,
    Permission::VoiceMute,
    Permission::VoicePriority,
    Permission::VoiceSpeak,
    Permission::VoiceVideo,
];

/// Default permissions for everyone in a trusted room (eg. with friends)
pub const EVERYONE_TRUSTED: &[Permission] = &[
    Permission::IntegrationsManage,
    Permission::UnusedEmojiAdd,
    Permission::EmojiUseExternal,
    Permission::InviteCreate,
    Permission::MessageCreate,
    Permission::MessageEmbeds,
    Permission::MessageMassMention, // maybe?
    Permission::MessageAttachments,
    Permission::MessageMove, // maybe?
    Permission::MessagePin,  // maybe?
    Permission::ReactionAdd,
    Permission::UnusedProfileAvatar,
    Permission::MemberNickname,
    Permission::TagApply,
    // Permission::TagManage, // maybe?
    Permission::ThreadArchive, // maybe?
    Permission::ThreadCreateChat,
    Permission::UnusedThreadCreateDocument,
    Permission::UnusedThreadCreateEvent,
    Permission::ThreadCreateForum,
    Permission::UnusedThreadCreateForum2,
    Permission::UnusedThreadCreateTable,
    Permission::ThreadCreateVoice,
    Permission::ThreadCreatePublic,
    Permission::ThreadCreatePrivate,
    Permission::ThreadEdit,    // maybe?
    Permission::ThreadForward, // maybe?
    // Permission::ThreadPin, // maybe?
    // Permission::ThreadPublish, // maybe?
    Permission::ViewAuditLog, // maybe?
    Permission::VoiceConnect,
    Permission::VoiceSpeak,
    Permission::VoiceVideo,
];

/// Default permissions for everyone in an untrusted room (eg. public)
pub const EVERYONE_UNTRUSTED: &[Permission] = &[
    Permission::EmojiUseExternal,
    Permission::InviteCreate,
    Permission::MessageCreate,
    Permission::MessageEmbeds,
    Permission::MessageAttachments,
    Permission::ReactionAdd,
    Permission::UnusedProfileAvatar,
    Permission::MemberNickname,
    Permission::TagApply, // maybe?
    Permission::ThreadCreateChat,
    Permission::UnusedThreadCreateDocument,
    Permission::UnusedThreadCreateEvent,
    Permission::ThreadCreateForum,
    Permission::UnusedThreadCreateForum2,
    Permission::UnusedThreadCreateTable,
    Permission::ThreadCreateVoice,
    Permission::ThreadCreatePublic,
    Permission::ThreadCreatePrivate,
    Permission::ThreadEdit,    // maybe?
    Permission::ThreadForward, // maybe?
    // Permission::ViewAuditLog, // maybe?
    Permission::VoiceConnect,
    Permission::VoiceSpeak,
    Permission::VoiceVideo,
];

/// extra permissions for someone who moderates stuff
pub const MODERATOR: &[Permission] = &[
    Permission::InviteManage,
    Permission::MemberBan,
    Permission::UnusedMemberBanManage,
    Permission::MemberKick,
    Permission::MemberNicknameManage,
    Permission::MessageDelete,
    Permission::MessageMove,
    Permission::MessagePin, // maybe?
    Permission::ReactionPurge,
    Permission::RoleApply, // maybe?
    Permission::TagApply,
    // Permission::TagManage, // maybe?
    Permission::ThreadArchive,
    Permission::ThreadRemove,
    Permission::ThreadEdit,
    Permission::ThreadForward, // maybe?
    Permission::ThreadLock,
    Permission::ThreadManage, // maybe?
    Permission::ViewAuditLog, // maybe?
    Permission::VoiceDeafen,
    Permission::VoiceDisconnect,
    Permission::VoiceMove,
    Permission::VoiceMute,
    Permission::VoicePriority, // maybe?
];
