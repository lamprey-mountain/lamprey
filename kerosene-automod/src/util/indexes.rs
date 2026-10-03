/// the index of an automod rule
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RuleIdx(u8);

/// a bitset containing indexes of automod rules
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RuleIndexes(u64);

impl TryFrom<u8> for RuleIdx {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value < 64 {
            Ok(Self(value))
        } else {
            Err(value)
        }
    }
}

impl From<RuleIdx> for u8 {
    fn from(v: RuleIdx) -> u8 {
        v.0
    }
}

impl std::ops::BitAnd for RuleIndexes {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl std::ops::BitAndAssign for RuleIndexes {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl std::ops::BitOr for RuleIndexes {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for RuleIndexes {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl std::ops::Not for RuleIndexes {
    type Output = Self;
    fn not(self) -> Self {
        Self(!self.0)
    }
}

impl RuleIndexes {
    /// create a new empty bitset
    #[inline]
    pub fn empty() -> Self {
        Self::default()
    }

    /// returns if no rules matched
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// iterate over contained rule indexes
    pub fn iter(&self) -> impl Iterator<Item = RuleIdx> {
        let mut bits = self.0;
        std::iter::from_fn(move || {
            if bits == 0 {
                return None;
            }

            let i = bits.trailing_zeros() as u8;
            bits &= bits - 1; // clear lowest set bit
            Some(RuleIdx(i))
        })
    }
}

impl From<u64> for RuleIndexes {
    fn from(v: u64) -> Self {
        Self(v)
    }
}
