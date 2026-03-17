use lamprey_macros::Diff;

/// Mock target type for testing
pub struct UserWithCache {
    pub name: String,
    pub description: Option<String>,
}

/// Patch struct with skipped field - infers target from name
#[derive(Diff)]
pub struct UserWithCachePatch {
    pub name: Option<String>,
    #[diff(skip)]
    pub internal_cache: Option<u64>,
    pub description: Option<String>,
}
