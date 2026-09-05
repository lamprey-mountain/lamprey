use crate::v2::types::components::ComponentId;
use thiserror::Error;

// TODO: impl and use these types

/// an error that occured while validating components
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("at least one root component is required")]
    MissingRoot,

    #[error("component {0:?} doesnt exist")]
    UnknownComponent(ComponentId),
    // etc...
}

/// an error that occured while applying a delta to some components
#[derive(Debug, Error)]
pub enum DeltaError {
    // etc...
}
