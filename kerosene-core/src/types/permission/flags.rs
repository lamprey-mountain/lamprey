/// extra flags for this state
///
/// - `1 << 0` cannot view (usually means "cannot view room")
/// - `1 << 1` room muted
/// - `1 << 2` room deafened
/// - `1 << 3` timed out
/// - `1 << 4` quarantined by automod
/// - `1 << 5` channel is locked
#[derive(Debug, Clone, Default)]
pub struct PermissionsFlags(u32);

impl PermissionsFlags {
    /// the user cannot view this resource
    ///
    /// this is used to return 404s instead of leaking that something exists
    const CANNOT_VIEW: u32 = 1 << 0;

    /// the user is voice muted in this room
    const ROOM_MUTED: u32 = 1 << 1;

    /// the user is voice deafened in this room
    const ROOM_DEAFENED: u32 = 1 << 2;

    /// the user is timed out in this room
    ///
    /// Removing all permissions except ViewChannel, ViewAuditLog, and
    /// ViewAnalytics. Also denies the ability to react entirely, even with
    /// existing reactions.
    const TIMED_OUT: u32 = 1 << 3;

    /// the user is quarantined by automod in this room
    ///
    /// similar effect as being timed out, but allows changing nickname
    const QUARANTINED: u32 = 1 << 4;

    /// this channel is locked. used to provide better error messages
    const CHANNEL_LOCKED: u32 = 1 << 5;

    #[inline]
    pub fn new() -> Self {
        Self(0)
    }

    #[inline]
    pub fn can_view(&self) -> bool {
        self.0 & Self::CANNOT_VIEW == 0
    }

    #[inline]
    pub fn set_can_view(&mut self) {
        self.0 &= !Self::CANNOT_VIEW;
    }

    #[inline]
    pub fn set_cannot_view(&mut self) {
        self.0 |= Self::CANNOT_VIEW;
    }

    #[inline]
    pub fn is_room_muted(&self) -> bool {
        self.0 & Self::ROOM_MUTED != 0
    }

    #[inline]
    pub fn set_room_muted(&mut self) {
        self.0 |= Self::ROOM_MUTED;
    }

    #[inline]
    pub fn set_not_room_muted(&mut self) {
        self.0 &= !Self::ROOM_MUTED;
    }

    #[inline]
    pub fn is_room_deafened(&self) -> bool {
        self.0 & Self::ROOM_DEAFENED != 0
    }

    #[inline]
    pub fn set_room_deafened(&mut self) {
        self.0 |= Self::ROOM_DEAFENED;
    }

    #[inline]
    pub fn set_not_room_deafened(&mut self) {
        self.0 &= !Self::ROOM_DEAFENED;
    }

    #[inline]
    pub fn is_timed_out(&self) -> bool {
        self.0 & Self::TIMED_OUT != 0
    }

    #[inline]
    pub fn set_timed_out(&mut self) {
        self.0 |= Self::TIMED_OUT;
    }

    #[inline]
    pub fn set_not_timed_out(&mut self) {
        self.0 &= !Self::TIMED_OUT;
    }

    #[inline]
    pub fn is_quarantined(&self) -> bool {
        self.0 & Self::QUARANTINED != 0
    }

    #[inline]
    pub fn set_quarantined(&mut self) {
        self.0 |= Self::QUARANTINED;
    }

    #[inline]
    pub fn set_not_quarantined(&mut self) {
        self.0 &= !Self::QUARANTINED;
    }

    #[inline]
    pub fn is_channel_locked(&self) -> bool {
        self.0 & Self::CHANNEL_LOCKED != 0
    }

    #[inline]
    pub fn set_channel_locked(&mut self) {
        self.0 |= Self::CHANNEL_LOCKED;
    }

    #[inline]
    pub fn set_not_channel_locked(&mut self) {
        self.0 &= !Self::CHANNEL_LOCKED;
    }
}
