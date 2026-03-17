use lamprey_macros::Diff;

/// Patch struct with skipped field
#[derive(Diff)]
pub struct UserPatchWithSkip {
    pub name: Option<String>,
    #[diff(skip)]
    pub internal_cache: Option<u64>,
    pub description: Option<String>,
}
