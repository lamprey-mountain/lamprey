pub use lamprey_macros::Diff;

pub trait Diff {
    type Target;

    /// whether this patch would change the other resource
    fn changes(&self, other: &Self::Target) -> bool;

    /// apply the patch to the target, returning the modified value
    fn apply(self, other: Self::Target) -> Self::Target;
}
