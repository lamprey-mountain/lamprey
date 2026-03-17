use lamprey_macros::Diff;

/// Unit struct (no fields)
#[derive(Diff)]
pub struct UnitPatch;

/// Empty struct with named fields
#[derive(Diff)]
pub struct EmptyPatch {}
