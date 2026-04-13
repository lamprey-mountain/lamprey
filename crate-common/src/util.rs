pub use lamprey_macros::Diff;

pub trait Diff {
    type Target;

    /// whether this patch would change the other resource
    fn changes(&self, other: &Self::Target) -> bool;

    /// apply the patch to the target, returning the modified value
    fn apply(self, other: Self::Target) -> Self::Target;
}

pub fn is_valid_hostname(s: &str) -> bool {
    if s.is_empty() || s.len() > 253 {
        return false;
    }

    // Optional trailing dot
    let s = s.strip_suffix('.').unwrap_or(s);

    if s.is_empty() {
        return false;
    }

    for label in s.split('.') {
        if label.is_empty() || label.len() > 63 {
            return false;
        }

        if label.starts_with('-') || label.ends_with('-') {
            return false;
        }

        if !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return false;
        }
    }

    true
}
