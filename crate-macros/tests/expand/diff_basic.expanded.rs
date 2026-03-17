use lamprey_macros::Diff;
/// Basic patch struct with Option fields
pub struct UserPatch {
    pub name: Option<String>,
    pub description: Option<String>,
    pub avatar: Option<u64>,
}
impl crate::v1::types::util::Diff<UserPatch> for UserPatch {
    fn changes(&self, other: &Self) -> bool {
        if let Some(ref val) = self.name {
            if val.changes(&other.name) {
                return true;
            }
        }
        if let Some(ref val) = self.description {
            if val.changes(&other.description) {
                return true;
            }
        }
        if let Some(ref val) = self.avatar {
            if val.changes(&other.avatar) {
                return true;
            }
        }
        false
    }
}
