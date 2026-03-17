use lamprey_macros::Diff;

/// Mock target type for testing
pub struct User {
    pub name: String,
    pub description: Option<String>,
    pub avatar: u64,
}

/// Basic patch struct - infers target from name (UserPatch -> User)
#[derive(Diff)]
pub struct UserPatch {
    pub name: Option<String>,
    pub description: Option<String>,
    pub avatar: Option<u64>,
}
