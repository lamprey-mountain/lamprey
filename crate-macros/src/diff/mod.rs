//! Diff derive macro implementation.

mod attr;
mod expand;

use syn::{parse_macro_input, DeriveInput};

/// Main entry point for the `#[derive(Diff)]` macro.
pub fn expand_diff_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let struct_ident = &input.ident;
    let generics = &input.generics;

    // Parse struct-level attributes
    let struct_attrs = match attr::DiffStructAttr::from_attrs(&input.attrs) {
        Ok(attrs) => attrs,
        Err(e) => return e.to_compile_error().into(),
    };

    match &input.data {
        syn::Data::Struct(data_struct) => {
            expand::expand_diff_derive(struct_ident, generics, data_struct, &struct_attrs)
        }
        syn::Data::Enum(enum_data) => {
            return syn::Error::new(
                enum_data.enum_token.span,
                "#[derive(Diff)]` does not support enums yet",
            )
            .to_compile_error()
            .into();
        }
        syn::Data::Union(union_data) => {
            return syn::Error::new(
                union_data.union_token.span,
                "`#[derive(Diff)]` does not support unions",
            )
            .to_compile_error()
            .into();
        }
    }
    .into()
}
