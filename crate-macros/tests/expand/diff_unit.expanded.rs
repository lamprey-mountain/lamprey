use lamprey_macros::Diff;
/// Mock target type for testing
pub struct Unit;
/// Mock target type for testing
pub struct Empty {}
/// Unit struct (no fields) - infers target from name
pub struct UnitPatch;
impl crate::v1::types::util::Diff<Unit> for UnitPatch {
    fn changes(&self, other: &Unit) -> bool {
        false
    }
}
/// Empty struct with named fields - infers target from name
pub struct EmptyPatch {}
impl crate::v1::types::util::Diff<Empty> for EmptyPatch {
    fn changes(&self, other: &Empty) -> bool {
        false
    }
}
