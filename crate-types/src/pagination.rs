use std::fmt::{self, Display};

use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

use super::Identifier;

#[derive(Debug, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PaginationQuery<I: Identifier> {
    pub from: Option<I>,
    pub to: Option<I>,
    pub dir: Option<PaginationDirection>,
    pub limit: Option<u16>,
}

// unfortunately, utoipa has issues with nested generics (Option<I>)
// TODO: better documentation generation
#[cfg(feature = "utoipa")]
impl<I: Identifier> IntoParams for PaginationQuery<I> {
    fn into_params(
        parameter_in_provider: impl Fn() -> Option<utoipa::openapi::path::ParameterIn>,
    ) -> Vec<utoipa::openapi::path::Parameter> {
        use utoipa::openapi::{path::ParameterBuilder, ObjectBuilder};
        let ident = ObjectBuilder::new()
            .schema_type(utoipa::openapi::schema::Type::String)
            .build();
        let limit = ObjectBuilder::new()
            .schema_type(utoipa::openapi::schema::Type::Integer)
            .minimum(Some(0))
            .maximum(Some(100))
            .default(Some(10.into()))
            .build();
        let dir = ObjectBuilder::new()
            .schema_type(utoipa::openapi::schema::Type::String)
            .enum_values(Some(["b", "f"]))
            .build();
        vec![
            ParameterBuilder::new()
                .name("from")
                .parameter_in(parameter_in_provider().unwrap_or_default())
                .schema(Some(ident.clone()))
                .build(),
            ParameterBuilder::new()
                .name("to")
                .parameter_in(parameter_in_provider().unwrap_or_default())
                .schema(Some(ident))
                .build(),
            ParameterBuilder::new()
                .name("dir")
                .parameter_in(parameter_in_provider().unwrap_or_default())
                .schema(Some(dir))
                .build(),
            ParameterBuilder::new()
                .name("limit")
                .parameter_in(parameter_in_provider().unwrap_or_default())
                .schema(Some(limit))
                .build(),
        ]
    }
}

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum PaginationDirection {
    #[default]
    F,
    B,
}

impl Display for PaginationDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PaginationDirection::F => write!(f, "f"),
            PaginationDirection::B => write!(f, "b"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PaginationResponse<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub has_more: bool,
}
