use common::v1::types::Permission;

/// compressed representation of permissions, for faster perm checks
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PermissionBits(u128);

/// permissions that affect one's ability to view something
pub const VIEW_PERMS: PermissionBits = PermissionBits(
    (1u128 << 39) | // ChannelView
    (1u128 << 40) | // AuditLogView
    (1u128 << 41), // AnalyticsView
);

/// permissions for lurkers in broadcast channels
pub const BROADCAST_LURKER_PERMS: PermissionBits = PermissionBits(
    (1u128 << 39) | // ChannelView
    (1u128 << 40) | // AuditLogView
    (1u128 << 41) | // AnalyticsView
    (1u128 << 51) | // VoiceRequest
    (1u128 << 50), // VoiceVad
);

/// permissions for quarantined users (view + nickname)
pub const QUARANTINE_PERMS: PermissionBits = PermissionBits(
    (1u128 << 39) | // ChannelView
    (1u128 << 40) | // AuditLogView
    (1u128 << 41) | // AnalyticsView
    (1u128 << 10), // MemberNickname
);

impl PermissionBits {
    /// Maximum number of permissions that can be represented (currently limited by u128)
    const MAX_PERMISSIONS: usize = 128;

    /// Convert a Permission to its corresponding bit position
    const fn permission_to_bit(permission: Permission) -> u32 {
        match permission {
            Permission::Admin => 0,
            Permission::IntegrationsManage => 1,
            Permission::IntegrationsBridge => 2,
            Permission::EmojiManage => 3,
            Permission::EmojiUseExternal => 4,
            Permission::InviteCreate => 5,
            Permission::InviteManage => 6,
            Permission::MemberBan => 7,
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
            Permission::ReactionManage => 22,
            Permission::RoleApply => 23,
            Permission::RoleManage => 24,
            Permission::RoomEdit => 25,
            Permission::ServerMaintenance => 26,
            Permission::ServerMetrics => 27,
            Permission::ServerOversee => 28,
            Permission::ChannelSlowmodeBypass => 29,
            Permission::ChannelEdit => 30,
            Permission::ChannelManage => 31,
            Permission::ThreadCreatePrivate => 32,
            Permission::ThreadCreatePublic => 33,
            Permission::ThreadManage => 34,
            Permission::ThreadEdit => 35,
            Permission::ChannelView => 36,
            Permission::AuditLogView => 37,
            Permission::AnalyticsView => 38,
            Permission::VoiceDeafen => 39,
            Permission::VoiceMove => 40,
            Permission::VoiceMute => 41,
            Permission::VoicePriority => 42,
            Permission::VoiceSpeak => 43,
            Permission::VoiceVideo => 44,
            Permission::VoiceVad => 45,
            Permission::VoiceRequest => 46,
            Permission::VoiceBroadcast => 47,
            Permission::CalendarEventCreate => 48,
            Permission::CalendarEventRsvp => 49,
            Permission::CalendarEventManage => 50,
            Permission::DocumentCreate => 51,
            Permission::DocumentEdit => 52,
            Permission::DocumentComment => 53,
            Permission::RoomCreate => 54,
            Permission::RoomManage => 55,
            Permission::UserManage => 56,
            Permission::UserManageSelf => 57,
            Permission::UserProfileSelf => 58,
            Permission::ApplicationCreate => 59,
            Permission::ApplicationManage => 60,
            Permission::DmCreate => 61,
            Permission::FriendCreate => 62,
            Permission::RoomJoin => 63,
            Permission::CallUpdate => 64,
            Permission::RoomJoinForce => 65,
        }
    }

    /// Convert a bit position back to a Permission
    fn bit_to_permission(bit: u32) -> Option<Permission> {
        match bit {
            0 => Some(Permission::Admin),
            1 => Some(Permission::IntegrationsManage),
            2 => Some(Permission::IntegrationsBridge),
            3 => Some(Permission::EmojiManage),
            4 => Some(Permission::EmojiUseExternal),
            5 => Some(Permission::InviteCreate),
            6 => Some(Permission::InviteManage),
            7 => Some(Permission::MemberBan),
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
            22 => Some(Permission::ReactionManage),
            23 => Some(Permission::RoleApply),
            24 => Some(Permission::RoleManage),
            25 => Some(Permission::RoomEdit),
            26 => Some(Permission::ServerMaintenance),
            27 => Some(Permission::ServerMetrics),
            28 => Some(Permission::ServerOversee),
            29 => Some(Permission::ChannelSlowmodeBypass),
            30 => Some(Permission::ChannelEdit),
            31 => Some(Permission::ChannelManage),
            32 => Some(Permission::ThreadCreatePrivate),
            33 => Some(Permission::ThreadCreatePublic),
            34 => Some(Permission::ThreadManage),
            35 => Some(Permission::ThreadEdit),
            36 => Some(Permission::ChannelView),
            37 => Some(Permission::AuditLogView),
            38 => Some(Permission::AnalyticsView),
            39 => Some(Permission::VoiceDeafen),
            40 => Some(Permission::VoiceMove),
            41 => Some(Permission::VoiceMute),
            42 => Some(Permission::VoicePriority),
            43 => Some(Permission::VoiceSpeak),
            44 => Some(Permission::VoiceVideo),
            45 => Some(Permission::VoiceVad),
            46 => Some(Permission::VoiceRequest),
            47 => Some(Permission::VoiceBroadcast),
            48 => Some(Permission::CalendarEventCreate),
            49 => Some(Permission::CalendarEventRsvp),
            50 => Some(Permission::CalendarEventManage),
            51 => Some(Permission::DocumentCreate),
            52 => Some(Permission::DocumentEdit),
            53 => Some(Permission::DocumentComment),
            54 => Some(Permission::RoomCreate),
            55 => Some(Permission::RoomManage),
            56 => Some(Permission::UserManage),
            57 => Some(Permission::UserManageSelf),
            58 => Some(Permission::UserProfileSelf),
            59 => Some(Permission::ApplicationCreate),
            60 => Some(Permission::ApplicationManage),
            61 => Some(Permission::DmCreate),
            62 => Some(Permission::FriendCreate),
            63 => Some(Permission::RoomJoin),
            64 => Some(Permission::CallUpdate),
            65 => Some(Permission::RoomJoinForce),
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

impl std::ops::BitOrAssign for PermissionBits {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl std::ops::BitAndAssign for PermissionBits {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl PermissionBits {
    /// Add all permissions from another PermissionBits
    #[inline]
    pub fn add_all(&mut self, other: PermissionBits) {
        self.0 |= other.0;
    }

    /// Remove all permissions that are set in another PermissionBits
    #[inline]
    pub fn remove_all(&mut self, other: PermissionBits) {
        self.0 &= !other.0;
    }

    /// Create a PermissionBits from a slice of Permissions
    pub fn from_slice(perms: &[Permission]) -> Self {
        let mut bits = PermissionBits(0);
        for perm in perms {
            bits.add(*perm);
        }
        bits
    }

    /// remove all permissions except those in the allowed set
    #[inline]
    pub fn mask(&mut self, mask: PermissionBits) {
        self.0 &= mask.0;
    }

    /// Check if any of the given permissions are set
    #[inline]
    pub fn has_any(&self, perms: &[Permission]) -> bool {
        let mask = Self::from_slice(perms);
        (self.0 & mask.0) != 0
    }

    /// Check if all of the given permissions are set
    #[inline]
    pub fn has_all(&self, perms: &[Permission]) -> bool {
        let mask = Self::from_slice(perms);
        (self.0 & mask.0) == mask.0
    }
}
