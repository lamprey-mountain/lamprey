use lamprey_macros::Diff;
/// Unit struct (no fields)
pub struct UnitPatch;
impl crate::v1::types::util::Diff<UnitPatch> for UnitPatch {
    fn changes(&self, other: &Self) -> bool {
        false
    }
}
/// Empty struct with named fields
pub struct EmptyPatch {}
impl crate::v1::types::util::Diff<EmptyPatch> for EmptyPatch {
    fn changes(&self, other: &Self) -> bool {
        false
    }
}
