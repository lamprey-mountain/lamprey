use lamprey_macros::Diff;
/// Patch struct with skipped field
pub struct UserPatchWithSkip {
    pub name: Option<String>,
    #[diff(skip)]
    pub internal_cache: Option<u64>,
    pub description: Option<String>,
}
impl crate::v1::types::util::Diff<UserPatchWithSkip> for UserPatchWithSkip {
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
        false
    }
}
