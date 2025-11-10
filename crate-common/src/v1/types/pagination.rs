use std::fmt::{self, Display};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

pub trait PaginationKey: Display + Clone + PartialEq + Eq + PartialOrd + Ord {
    fn min() -> Self;
    fn max() -> Self;
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PaginationQuery<K: PaginationKey> {
    /// The key to start paginating from. Not inclusive. Optional.
    pub from: Option<K>,

    /// The key to stop paginating at. Not inclusive. Optional.
    pub to: Option<K>,

    /// Whether to paginate forwards or backwards. Defaults to forwards. Paginating backwards does not reverse the order of items.
    pub dir: Option<PaginationDirection>,

    /// The maximum number of items to return.
    pub limit: Option<u16>,
}

// unfortunately, utoipa has issues with nested generics (Option<I>)
// TODO: better documentation generation
#[cfg(feature = "utoipa")]
impl<K: PaginationKey> IntoParams for PaginationQuery<K> {
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
            .maximum(Some(1024))
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

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
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
    pub cursor: Option<String>,
}

// pub trait HasKey {
//     type PaginationKey: PaginationKey;
// }

// #[derive(Debug, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// pub struct PaginationResponse2<T: core::fmt::Debug + Serialize + Deserialize<'_> + HasKey> {
//     pub items: Vec<T>,
//     pub total: u64,
//     pub after: Option<T::PaginationKey>,
//     pub before: Option<T::PaginationKey>,
// }
