//! Code generation for the Diff derive macro.

use darling::FromField;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DataStruct, Generics, Ident};

use super::attr::{DiffFieldAttr, DiffStructAttr};

/// Generate the `Diff` trait implementation for a struct.
pub fn expand_diff_derive(
    struct_ident: &Ident,
    generics: &Generics,
    data_struct: &DataStruct,
    attrs: &DiffStructAttr,
) -> TokenStream {
    // Determine target type
    let target_ty = if let Some(target) = &attrs.target {
        // Parse the target type string
        match target.parse::<syn::Type>() {
            Ok(ty) => ty,
            Err(e) => return e.to_compile_error(),
        }
    } else {
        // Default: try to infer from struct name by removing "Patch" suffix
        let struct_name = struct_ident.to_string();
        if let Some(base_name) = struct_name.strip_suffix("Patch") {
            match syn::parse_str::<syn::Type>(base_name) {
                Ok(ty) => ty,
                Err(_) => {
                    return syn::Error::new(
                        struct_ident.span(),
                        format!(
                            "Could not infer target type from '{}'. Please specify #[diff(target = \"TypeName\")]",
                            struct_name
                        ),
                    )
                    .to_compile_error();
                }
            }
        } else {
            return syn::Error::new(
                struct_ident.span(),
                "Could not infer target type. Please specify #[diff(target = \"TypeName\")]",
            )
            .to_compile_error();
        }
    };

    // Extract field comparison logic
    let field_checks = generate_field_checks(data_struct);

    // Split generics for impl
    let (impl_generics, ty_generics, _where_clause) = generics.split_for_impl();

    // Generate where clause bounds for Diff trait
    let diff_where_clause = generate_diff_where_clause(generics, data_struct);

    quote! {
        impl #impl_generics crate::v1::types::util::Diff<#target_ty> for #struct_ident #ty_generics
        #diff_where_clause
        {
            fn changes(&self, other: &#target_ty) -> bool {
                #field_checks
            }
        }
    }
}

/// Generate the field comparison code.
fn generate_field_checks(data_struct: &DataStruct) -> TokenStream {
    let fields = match &data_struct.fields {
        syn::Fields::Named(fields) => &fields.named,
        syn::Fields::Unnamed(fields) => &fields.unnamed,
        syn::Fields::Unit => return quote! { false },
    };

    let mut checks = Vec::new();

    for (index, field) in fields.iter().enumerate() {
        // Parse field attributes
        let field_attr = match DiffFieldAttr::from_field(&field) {
            Ok(attr) => attr,
            Err(e) => {
                return e.write_errors().into();
            }
        };

        // Skip fields marked with #[diff(skip)]
        if field_attr.skip {
            continue;
        }

        let field_ident = field
            .ident
            .as_ref()
            .map(|ident| quote! { #ident })
            .unwrap_or_else(|| {
                let index = syn::Index::from(index);
                quote! { #index }
            });

        let field_ty = &field.ty;

        // Generate comparison based on field type
        let check = generate_field_check(&field_ident, field_ty);
        checks.push(check);
    }

    if checks.is_empty() {
        quote! { false }
    } else {
        quote! {
            #(#checks)*
            false
        }
    }
}

/// Generate comparison code for a single field.
fn generate_field_check(field_ident: &TokenStream, field_ty: &syn::Type) -> TokenStream {
    // Check if the field type is Option<T>
    if extract_option_inner(field_ty).is_some() {
        // For Option<T>, only check if Some
        quote! {
            if let Some(ref val) = self.#field_ident {
                if val.changes(&other.#field_ident) {
                    return true;
                }
            }
        }
    } else {
        // For direct types, compare directly
        quote! {
            if self.#field_ident.changes(&other.#field_ident) {
                return true;
            }
        }
    }
}

/// Extract the inner type from an Option<T>.
fn extract_option_inner(ty: &syn::Type) -> Option<&syn::Type> {
    match ty {
        syn::Type::Path(type_path) => {
            let path = &type_path.path;
            if path.leading_colon.is_some() {
                return None;
            }
            if path.segments.len() != 1 {
                return None;
            }
            let segment = path.segments.first()?;
            if segment.ident != "Option" {
                return None;
            }
            match &segment.arguments {
                syn::PathArguments::AngleBracketed(args) => {
                    if args.args.len() != 1 {
                        return None;
                    }
                    match args.args.first()? {
                        syn::GenericArgument::Type(inner_ty) => Some(inner_ty),
                        _ => None,
                    }
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// Generate additional where clause bounds for Diff trait.
fn generate_diff_where_clause(
    generics: &Generics,
    _data_struct: &DataStruct,
) -> Option<syn::WhereClause> {
    if generics.params.is_empty() {
        return None;
    }

    let mut where_clause = generics
        .where_clause
        .clone()
        .unwrap_or_else(|| syn::WhereClause {
            where_token: Default::default(),
            predicates: Default::default(),
        });

    // Add Diff bounds for each type parameter
    for param in &generics.params {
        if let syn::GenericParam::Type(type_param) = param {
            let ident = &type_param.ident;
            where_clause
                .predicates
                .push(syn::parse_quote! { #ident: crate::v1::types::util::Diff<#ident> });
        }
    }

    Some(where_clause)
}
