use crate::{
    v1::types::error::ApiResult,
    v2::types::components::{Component, ComponentType, Components},
};

impl ComponentType {
    pub fn validate(&self) -> ApiResult<()> {
        todo!()
    }
}

impl Components {
    pub fn validate(&self) -> ApiResult<()> {
        todo!()
    }
}

impl Component {
    pub fn validate(&self) -> ApiResult<()> {
        todo!()
    }
}

// TODO: validate Action for component
// TODO: validate Validation for component
