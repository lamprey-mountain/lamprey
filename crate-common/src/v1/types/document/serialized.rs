//! json serialized document format

// TODO: actually flesh out the serialized document format

use lamprey_macros::record;

use crate::{util::registry::export_models, v1::types::components::ComponentCanonical};

/// serialized document
#[record]
#[derive(PartialEq)]
pub struct Serdoc {
    pub components: Vec<ComponentCanonical>,
}

export_models!(Serdoc);
