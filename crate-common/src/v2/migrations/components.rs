// TODO: conversions between v1 components and v2 components

use crate::v1::types::components as v1;
use crate::v2::types::components as v2;

impl From<v1::ComponentsCanonical> for v2::Components {
    fn from(value: v1::ComponentsCanonical) -> Self {
        todo!()
    }
}

impl From<v2::Components> for v1::ComponentsCanonical {
    fn from(value: v2::Components) -> Self {
        todo!()
    }
}
