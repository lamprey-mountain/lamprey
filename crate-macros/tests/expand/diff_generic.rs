use lamprey_macros::Diff;

/// Mock target type for testing
pub struct GenericTarget<T> {
    pub value: T,
}

/// Generic patch struct - infers target from name
#[derive(Diff)]
pub struct GenericPatch<T> {
    pub value: Option<T>,
}

/// Generic patch with where clause
#[derive(Diff)]
pub struct GenericPatchWithBounds<T>
where
    T: Clone,
{
    pub value: Option<T>,
}
