//! Attribute parsing for the Diff derive macro.

use syn::{
    parse::{Parse, ParseStream},
    Attribute, Ident, LitStr, Meta, Result,
};

/// Represents the `#[diff(...)]` attribute on a struct.
#[derive(Debug, Clone, Default)]
pub struct DiffStructAttr {
    pub target: Option<LitStr>,
}

impl DiffStructAttr {
    /// Parse attributes from a struct to extract `#[diff(...)]` metadata.
    pub fn from_attrs(attrs: &[Attribute]) -> Result<Self> {
        let mut result = DiffStructAttr::default();

        for attr in attrs {
            if !attr.path().is_ident("diff") {
                continue;
            }

            match &attr.meta {
                Meta::List(list) => {
                    let nested = list.parse_args::<DiffStructAttrInner>()?;
                    if let Some(target) = nested.target {
                        result.target = Some(target);
                    }
                }
                Meta::NameValue(name_value) => {
                    if name_value.path.is_ident("target") {
                        if let syn::Expr::Lit(expr_lit) = &name_value.value {
                            if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                                result.target = Some(lit_str.clone());
                            }
                        }
                    }
                }
                Meta::Path(path) => {
                    return Err(syn::Error::new_spanned(
                        path,
                        "#[diff] attribute must have content, e.g., #[diff(target = \"User\")]",
                    ));
                }
            }
        }

        Ok(result)
    }
}

/// Internal representation for parsing the content inside `#[diff(...)]` on structs.
#[derive(Debug, Clone)]
struct DiffStructAttrInner {
    target: Option<LitStr>,
}

impl Parse for DiffStructAttrInner {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut target = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            let ident_str = ident.to_string();

            if ident_str == "target" {
                input.parse::<syn::Token![=]>()?;
                target = Some(input.parse::<LitStr>()?);
            } else {
                return Err(syn::Error::new(
                    ident.span(),
                    format!("unknown #[diff] attribute: `{ident}`. Supported: `target`"),
                ));
            }

            // Parse trailing comma if present
            if input.peek(syn::Token![,]) {
                input.parse::<syn::Token![,]>()?;
            }
        }

        Ok(Self { target })
    }
}

/// Represents the `#[diff(...)]` attribute on a field.
#[derive(Debug, Clone, Default)]
pub struct DiffFieldAttr {
    pub skip: bool,
}

impl DiffFieldAttr {
    /// Parse attributes from a field to extract `#[diff(...)]` metadata.
    pub fn from_attrs(attrs: &[Attribute]) -> Result<Self> {
        let mut result = DiffFieldAttr::default();

        for attr in attrs {
            if !attr.path().is_ident("diff") {
                continue;
            }

            match &attr.meta {
                Meta::Path(path) => {
                    return Err(syn::Error::new_spanned(
                        path,
                        "#[diff] attribute must have content, e.g., #[diff(skip)]",
                    ));
                }
                Meta::List(list) => {
                    let nested = list.parse_args::<DiffFieldAttrInner>()?;
                    if nested.skip {
                        result.skip = true;
                    }
                }
                Meta::NameValue(_) => {
                    return Err(syn::Error::new_spanned(
                        attr,
                        "#[diff] attribute does not support name-value syntax",
                    ));
                }
            }
        }

        Ok(result)
    }
}

/// Internal representation for parsing the content inside `#[diff(...)]` on fields.
#[derive(Debug, Clone)]
struct DiffFieldAttrInner {
    skip: bool,
}

impl Parse for DiffFieldAttrInner {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut skip = false;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;

            match ident.to_string().as_str() {
                "skip" => {
                    skip = true;
                }
                _ => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!("unknown #[diff] attribute: `{ident}`. Supported: `skip`"),
                    ));
                }
            }

            // Parse trailing comma if present
            if input.peek(syn::Token![,]) {
                input.parse::<syn::Token![,]>()?;
            }
        }

        Ok(Self { skip })
    }
}
