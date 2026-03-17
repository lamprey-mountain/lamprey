use lamprey_macros::Diff;
/// Mock target type for testing
pub struct GenericTarget<T> {
    pub value: T,
}
/// Generic patch struct - infers target from name
pub struct GenericPatch<T> {
    pub value: Option<T>,
}
impl<T> crate::v1::types::util::Diff<Generic> for GenericPatch<T>
where
    T: crate::v1::types::util::Diff<T>,
{
    fn changes(&self, other: &Generic) -> bool {
        if let Some(ref val) = self.value {
            if val.changes(&other.value) {
                return true;
            }
        }
        false
    }
}
/// Generic patch with where clause
pub struct GenericPatchWithBounds<T>
where
    T: Clone,
{
    pub value: Option<T>,
}
