use lamprey_macros::Diff;

/// Generic patch struct
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
