use lamprey_macros::Diff;

/// Basic patch struct with Option fields
#[derive(Diff)]
pub struct UserPatch {
    pub name: Option<String>,
    pub description: Option<String>,
    pub avatar: Option<u64>,
}
