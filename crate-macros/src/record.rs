use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Fields, parse::Parse, parse2};

/// macro attrs for a record
pub struct RecordArgs {
    // TODO
}

pub fn expand(args: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    let _args: RecordArgs = parse2(args)?;
    let mut input: DeriveInput = parse2(input)?;
    // let mut input = parse_macro_input!(input as DeriveInput);

    let is_struct = matches!(input.data, Data::Struct(_));

    wrap_attributes(&mut input.attrs);
    match &mut input.data {
        Data::Struct(data_struct) => {
            if let Fields::Named(fields) = &mut data_struct.fields {
                for field in fields.named.iter_mut() {
                    wrap_attributes(&mut field.attrs);
                }
            }
        }
        Data::Enum(data_enum) => {
            for variant in data_enum.variants.iter_mut() {
                wrap_attributes(&mut variant.attrs);
                if let Fields::Named(fields) = &mut variant.fields {
                    for field in fields.named.iter_mut() {
                        wrap_attributes(&mut field.attrs);
                    }
                }
            }
        }
        Data::Union(_) => return Err(syn::Error::new_spanned(input, "Unions are not supported")),
    }

    let validate_attr = if is_struct {
        quote! { #[cfg_attr(feature = "validator", derive(::validator::Validate))] }
    } else {
        quote! {}
    };

    let expanded = quote! {
        #[derive(Debug, Clone)]
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "utoipa", derive(::utoipa::ToSchema))]
        #validate_attr
        #input
    };

    Ok(TokenStream::from(expanded))
}

impl Parse for RecordArgs {
    fn parse(_input: syn::parse::ParseStream) -> syn::Result<Self> {
        // TODO
        Ok(RecordArgs {})
    }
}

/// wrap `#[attr_name(...)]` in `#[cfg_attr(feature = "feature_name", attr_name(...))]`
fn wrap_attribute(attrs: &mut Vec<Attribute>, attr_name: &str, feature_name: &str) {
    for attr in attrs.iter_mut() {
        if attr.path().is_ident(attr_name) {
            let meta = &attr.meta;
            let new_attr: Attribute = syn::parse_quote! {
                #[cfg_attr(feature = #feature_name, #meta)]
            };
            *attr = new_attr;
        }
    }
}

fn wrap_attributes(attrs: &mut Vec<Attribute>) {
    wrap_attribute(attrs, "serde", "serde");
    wrap_attribute(attrs, "validator", "validator");
    wrap_attribute(attrs, "utoipa", "utoipa");
    wrap_attribute(attrs, "schema", "utoipa");
}
