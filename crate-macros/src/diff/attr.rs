//! Attribute parsing for the Diff derive macro.

use darling::{FromDeriveInput, FromField};
use syn::LitStr;

/// Represents the `#[diff(...)]` attribute on the struct.
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(diff), forward_attrs(allow, doc, cfg))]
pub struct DiffStructAttr {
    pub target: Option<LitStr>,
}

/// Represents the `#[diff(...)]` attribute on a field.
#[derive(Debug, FromField)]
#[darling(attributes(diff), forward_attrs(allow, doc, cfg))]
pub struct DiffFieldAttr {
    #[darling(default)]
    pub skip: bool,
}
