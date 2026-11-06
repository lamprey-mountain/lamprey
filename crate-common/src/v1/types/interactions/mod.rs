use crate::util::registry::export_models;

pub mod create;
pub mod error;
pub mod inner;
pub mod response;

pub use create::{Create as InteractionCreate, CreateType as InteractionCreateType};
pub use error::ErrorCode as InteractionErrorCode;
pub use inner::{Interaction, InteractionType};
pub use response::{
    Create as InteractionResponseCreate, CreateType as InteractionResponseCreateType,
    Response as InteractionResponse,
};

export_models!(use create, use error, use inner, use response);
