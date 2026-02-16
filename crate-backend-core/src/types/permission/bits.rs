use common::v1::types::Permission;

/// compressed representation of permissions, for faster perm checks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermissionBits(u128);

impl PermissionBits {
    /// Maximum number of permissions that can be represented (currently limited by u128)
    const MAX_PERMISSIONS: usize = 128;

    /// Convert a Permission to its corresponding bit position
    fn permission_to_bit(permission: Permission) -> u32 {
        match permission {
            Permission::Admin => 0,
            Permission::IntegrationsManage => 1,
            Permission::EmojiManage => 2,
            Permission::EmojiUseExternal => 3,
            Permission::InviteCreate => 4,
            Permission::InviteManage => 5,
            Permission::MemberBan => 6,
            Permission::MemberBridge => 7,
            Permission::MemberKick => 8,
            Permission::MemberNicknameManage => 9,
            Permission::MemberNickname => 10,
            Permission::MemberTimeout => 11,
            Permission::MessageAttachments => 12,
            Permission::MessageCreate => 13,
            Permission::MessageCreateThread => 14,
            Permission::MessageDelete => 15,
            Permission::MessageRemove => 16,
            Permission::MessageEmbeds => 17,
            Permission::MessageMassMention => 18,
            Permission::MessageMove => 19,
            Permission::MessagePin => 20,
            Permission::ReactionAdd => 21,
            Permission::ReactionPurge => 22,
            Permission::RoleApply => 23,
            Permission::RoleManage => 24,
            Permission::RoomManage => 25,
            Permission::ServerMetrics => 26,
            Permission::ServerOversee => 27,
            Permission::ServerReports => 28,
            Permission::TagApply => 29,
            Permission::TagManage => 30,
            Permission::BypassSlowmode => 31,
            Permission::ChannelEdit => 32,
            Permission::ChannelManage => 33,
            Permission::ThreadCreatePrivate => 34,
            Permission::ThreadCreatePublic => 35,
            Permission::ThreadManage => 36,
            Permission::ThreadEdit => 37,
            Permission::ThreadLock => 38,
            Permission::ViewChannel => 39,
            Permission::ViewAuditLog => 40,
            Permission::ViewAnalytics => 41,
            Permission::VoiceConnect => 42,
            Permission::VoiceDeafen => 43,
            Permission::VoiceDisconnect => 44,
            Permission::VoiceMove => 45,
            Permission::VoiceMute => 46,
            Permission::VoicePriority => 47,
            Permission::VoiceSpeak => 48,
            Permission::VoiceVideo => 49,
            Permission::VoiceVad => 50,
            Permission::VoiceRequest => 51,
            Permission::VoiceBroadcast => 52,
            Permission::CalendarEventCreate => 53,
            Permission::CalendarEventRsvp => 54,
            Permission::CalendarEventManage => 55,
            Permission::DocumentCreate => 56,
            Permission::DocumentEdit => 57,
            Permission::DocumentComment => 58,
            Permission::RoomCreate => 59,
            Permission::RoomManageServer => 60,
            Permission::UserManage => 61,
            Permission::UserDeleteSelf => 62,
            Permission::UserProfile => 63,
            Permission::ApplicationCreate => 64,
            Permission::ApplicationManage => 65,
            Permission::DmCreate => 66,
            Permission::FriendCreate => 67,
            Permission::RoomJoin => 68,
            Permission::CallUpdate => 69,
            Permission::RoomForceJoin => 70,
        }
    }

    /// Convert a bit position back to a Permission
    fn bit_to_permission(bit: u32) -> Option<Permission> {
        match bit {
            0 => Some(Permission::Admin),
            1 => Some(Permission::IntegrationsManage),
            2 => Some(Permission::EmojiManage),
            3 => Some(Permission::EmojiUseExternal),
            4 => Some(Permission::InviteCreate),
            5 => Some(Permission::InviteManage),
            6 => Some(Permission::MemberBan),
            7 => Some(Permission::MemberBridge),
            8 => Some(Permission::MemberKick),
            9 => Some(Permission::MemberNicknameManage),
            10 => Some(Permission::MemberNickname),
            11 => Some(Permission::MemberTimeout),
            12 => Some(Permission::MessageAttachments),
            13 => Some(Permission::MessageCreate),
            14 => Some(Permission::MessageCreateThread),
            15 => Some(Permission::MessageDelete),
            16 => Some(Permission::MessageRemove),
            17 => Some(Permission::MessageEmbeds),
            18 => Some(Permission::MessageMassMention),
            19 => Some(Permission::MessageMove),
            20 => Some(Permission::MessagePin),
            21 => Some(Permission::ReactionAdd),
            22 => Some(Permission::ReactionPurge),
            23 => Some(Permission::RoleApply),
            24 => Some(Permission::RoleManage),
            25 => Some(Permission::RoomManage),
            26 => Some(Permission::ServerMetrics),
            27 => Some(Permission::ServerOversee),
            28 => Some(Permission::ServerReports),
            29 => Some(Permission::TagApply),
            30 => Some(Permission::TagManage),
            31 => Some(Permission::BypassSlowmode),
            32 => Some(Permission::ChannelEdit),
            33 => Some(Permission::ChannelManage),
            34 => Some(Permission::ThreadCreatePrivate),
            35 => Some(Permission::ThreadCreatePublic),
            36 => Some(Permission::ThreadManage),
            37 => Some(Permission::ThreadEdit),
            38 => Some(Permission::ThreadLock),
            39 => Some(Permission::ViewChannel),
            40 => Some(Permission::ViewAuditLog),
            41 => Some(Permission::ViewAnalytics),
            42 => Some(Permission::VoiceConnect),
            43 => Some(Permission::VoiceDeafen),
            44 => Some(Permission::VoiceDisconnect),
            45 => Some(Permission::VoiceMove),
            46 => Some(Permission::VoiceMute),
            47 => Some(Permission::VoicePriority),
            48 => Some(Permission::VoiceSpeak),
            49 => Some(Permission::VoiceVideo),
            50 => Some(Permission::VoiceVad),
            51 => Some(Permission::VoiceRequest),
            52 => Some(Permission::VoiceBroadcast),
            53 => Some(Permission::CalendarEventCreate),
            54 => Some(Permission::CalendarEventRsvp),
            55 => Some(Permission::CalendarEventManage),
            56 => Some(Permission::DocumentCreate),
            57 => Some(Permission::DocumentEdit),
            58 => Some(Permission::DocumentComment),
            59 => Some(Permission::RoomCreate),
            60 => Some(Permission::RoomManageServer),
            61 => Some(Permission::UserManage),
            62 => Some(Permission::UserDeleteSelf),
            63 => Some(Permission::UserProfile),
            64 => Some(Permission::ApplicationCreate),
            65 => Some(Permission::ApplicationManage),
            66 => Some(Permission::DmCreate),
            67 => Some(Permission::FriendCreate),
            68 => Some(Permission::RoomJoin),
            69 => Some(Permission::CallUpdate),
            70 => Some(Permission::RoomForceJoin),
            _ => None,
        }
    }

    /// Check if a specific permission is set
    pub fn has(&self, permission: Permission) -> bool {
        let bit_pos = Self::permission_to_bit(permission);
        let bit_mask = 1u128 << bit_pos;
        (self.0 & bit_mask) != 0
    }

    /// Add a permission
    pub fn add(&mut self, permission: Permission) {
        let bit_pos = Self::permission_to_bit(permission);
        let bit_mask = 1u128 << bit_pos;
        self.0 |= bit_mask;
    }

    /// Remove a permission
    pub fn remove(&mut self, permission: Permission) {
        let bit_pos = Self::permission_to_bit(permission);
        let bit_mask = 1u128 << bit_pos;
        self.0 &= !bit_mask;
    }

    /// Create PermissionBits from raw u128 value
    pub fn from_bits(bits: u128) -> Self {
        PermissionBits(bits)
    }

    /// Get the raw u128 value
    pub fn into_bits(self) -> u128 {
        self.0
    }

    /// Get all permissions contained in this PermissionBits
    pub fn to_vec(&self) -> Vec<Permission> {
        let mut permissions = Vec::new();
        for i in 0..Self::MAX_PERMISSIONS as u32 {
            if let Some(permission) = Self::bit_to_permission(i) {
                if self.has(permission) {
                    permissions.push(permission);
                }
            }
        }
        permissions
    }
}

impl From<Vec<Permission>> for PermissionBits {
    fn from(value: Vec<Permission>) -> Self {
        let mut bits = PermissionBits(0);
        for permission in value {
            bits.add(permission);
        }
        bits
    }
}

impl From<&Vec<Permission>> for PermissionBits {
    fn from(value: &Vec<Permission>) -> Self {
        let mut bits = PermissionBits(0);
        for permission in value {
            bits.add(*permission);
        }
        bits
    }
}

impl From<PermissionBits> for Vec<Permission> {
    fn from(value: PermissionBits) -> Self {
        value.to_vec()
    }
}

impl From<Permission> for PermissionBits {
    fn from(permission: Permission) -> Self {
        let mut bits = PermissionBits(0);
        bits.add(permission);
        bits
    }
}

impl Default for PermissionBits {
    fn default() -> Self {
        PermissionBits(0)
    }
}

impl std::ops::BitOr for PermissionBits {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        PermissionBits(self.0 | rhs.0)
    }
}

impl std::ops::BitAnd for PermissionBits {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        PermissionBits(self.0 & rhs.0)
    }
}

impl std::ops::BitXor for PermissionBits {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        PermissionBits(self.0 ^ rhs.0)
    }
}

impl std::ops::Not for PermissionBits {
    type Output = Self;

    fn not(self) -> Self::Output {
        PermissionBits(!self.0)
    }
}
