use lamprey_macros::record;

use crate::v2::types::components::{ComponentId, Components};

#[record]
pub struct ComponentDeltaCreate {
    // ...
}

#[record]
pub struct ComponentDelta {
    pub components: Vec<Component>,

    pub init: Option<Components>,
    pub append: Vec<ComponentAppend>,
    pub replace: Vec<ComponentReplace>,
    pub delete: Vec<ComponentId>,
}

pub struct ComponentAppend {
    pub target: ComponentId,
    pub component_ids: Vec<ComponentId>,
    // pub where: String, // summary
}

pub struct ComponentReplace {
    pub target: ComponentId,
    pub component_ids: Vec<ComponentId>,
}
