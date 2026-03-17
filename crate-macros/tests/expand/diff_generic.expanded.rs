use lamprey_macros::Diff;
/// Generic patch struct
pub struct GenericPatch<T> {
    pub value: Option<T>,
}
impl<T> crate::v1::types::util::Diff<GenericPatch<T>> for GenericPatch<T>
where
    T: crate::v1::types::util::Diff<T>,
{
    fn changes(&self, other: &Self) -> bool {
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
impl<T> crate::v1::types::util::Diff<GenericPatchWithBounds<T>>
for GenericPatchWithBounds<T>
where
    T: Clone,
    T: crate::v1::types::util::Diff<T>,
{
    fn changes(&self, other: &Self) -> bool {
        if let Some(ref val) = self.value {
            if val.changes(&other.value) {
                return true;
            }
        }
        false
    }
}
