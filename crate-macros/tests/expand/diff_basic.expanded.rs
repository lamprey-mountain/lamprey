use lamprey_macros::Diff;
/// Mock target type for testing
pub struct User {
    pub name: String,
    pub description: Option<String>,
    pub avatar: u64,
}
/// Basic patch struct - infers target from name (UserPatch -> User)
pub struct UserPatch {
    pub name: Option<String>,
    pub description: Option<String>,
    pub avatar: Option<u64>,
}
impl crate::v1::types::util::Diff<User> for UserPatch {
    fn changes(&self, other: &User) -> bool {
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
