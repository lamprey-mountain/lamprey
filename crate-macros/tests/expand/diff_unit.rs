use lamprey_macros::Diff;

/// Mock target type for testing
pub struct Unit;

/// Mock target type for testing
pub struct Empty {}

/// Unit struct (no fields) - infers target from name
#[derive(Diff)]
pub struct UnitPatch;

/// Empty struct with named fields - infers target from name
#[derive(Diff)]
pub struct EmptyPatch {}
