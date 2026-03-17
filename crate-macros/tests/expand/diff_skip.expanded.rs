use lamprey_macros::Diff;
/// Mock target type for testing
pub struct UserWithCache {
    pub name: String,
    pub description: Option<String>,
}
/// Patch struct with skipped field - infers target from name
pub struct UserWithCachePatch {
    pub name: Option<String>,
    #[diff(skip)]
    pub internal_cache: Option<u64>,
    pub description: Option<String>,
}
impl crate::v1::types::util::Diff<UserWithCache> for UserWithCachePatch {
    fn changes(&self, other: &UserWithCache) -> bool {
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
