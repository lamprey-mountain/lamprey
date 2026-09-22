use lamprey_macros::record;

/// filter to only certain event kinds
#[record]
#[derive(Default)]
pub struct DispatchFilter {
    // TODO: design this
    // NOTE: may not be needed with the new subscription system
}
