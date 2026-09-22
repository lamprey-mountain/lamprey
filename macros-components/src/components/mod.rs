use lamprey::v1::types::components::IdAllocator;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, ExprLit, Ident, Lit, parse2};

use crate::{
    color,
    components::parse::{Arg, Children, Component},
    util::common_crate,
};

mod parse;

pub fn expand(item: TokenStream) -> syn::Result<TokenStream> {
    let parsed: Children = parse2(item)?;

    let krate = common_crate();

    let mut id_alloc = IdAllocator::new();
    let mut items = Vec::new();
    let mut root_ids = Vec::new();

    for component in parsed.components {
        items.push(component.expand(&krate, true)?);
    }

    let push_item_stmts = items;

    // TODO: defaults for fields (eg. default container/section/details color to None, details open to false, etc...)
    // TODO: imply format!() for text(...)
    // TODO: but also let text(...) accept anything that impls Display, Option

    // future work
    // TODO: accept iterators for lists/children components
    // TODO: maybe add pseudo components to generate markdown? (table, ul, ol)
    // TODO: component update syntax? maybe with an option to construct FlumeDelta?

    Ok(quote! {
        {
            let mut id_alloc = #krate::v1::types::components::IdAllocator::new();
            let mut components = #krate::v2::types::components::components::Components::default();
            #(#push_item_stmts)*
            components
        }
    })
}

impl Component {
    /// look up an arg by name
    fn arg(&self, name: &str) -> Option<&Expr> {
        self.args
            .iter()
            .find(|a| a.name.to_string() == name)
            .map(|a| &a.value)
    }

    fn expand(self, krate: &TokenStream, is_root: bool) -> syn::Result<TokenStream> {
        let ty = map_component_type(self.name.clone());
        let ty_str = ty.to_string();

        let id_expr = self
            .arg("id")
            .cloned()
            .map(|e| quote! { Some(#e) })
            .unwrap_or_else(|| quote! { None });

        let allow = self
            .arg("allow")
            .cloned()
            .unwrap_or_else(|| syn::parse_quote!(None));

        let attr_fields = self
            .args
            .iter()
            .filter(|a| {
                let n = a.name.to_string();
                n != "id" && n != "allow"
            })
            .map(|a| {
                let resolved = a.resolve(&ty_str)?;
                let name = &a.name;
                Ok(quote! { #name: #resolved.into() })
            })
            .collect::<syn::Result<Vec<_>>>()?;

        let mut sections = Vec::new();
        for group in self.children {
            let section = group.section.unwrap_or_else(|| format_ident!("children"));

            let mut child_ids = Vec::new();
            for component in group.components {
                let tokens = component.expand(krate, false)?;
                // FIXME: push children to section
            }

            // if let Some((_, existing)) = sections.iter_mut().find(|(s, _)| *s == section) {
            //     existing.extend(child_ids);
            // } else {
            //     sections.push((section, child_ids));
            // }
        }

        // let section_fields = sections.into_iter().map(|(section, ids)| {
        //     let id_exprs = ids.iter().map(|id| {
        //         quote! { #krate::v2::types::components::ComponentId(#id) }
        //     });
        //     quote! { #section: vec![ #(#id_exprs),* ] }
        // });

        let push_root = if is_root {
            quote! { components.roots.push(id); }
        } else {
            quote! {}
        };

        Ok(quote! {
            let id = id_alloc.try_allocate(#id_expr).unwrap();
            #push_root
            components.items.push(
                #krate::v2::types::components::Component {
                    id,
                    ty: #krate::v2::types::components::ComponentType::#ty {
                        #(#attr_fields,)*
                        #(#section_fields,)*
                    },
                    allow: #allow,
                }
            );
        })
    }
}

impl Arg {
    fn resolve(&self, ty: &str) -> syn::Result<TokenStream> {
        let value = self.value.clone();
        match (self.name.to_string().as_str(), ty) {
            ("style", "Button") => Ok(map_button_style(quote! { #value })),
            ("color", "Container" | "Details" | "Section") => match value {
                Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) => color::expand_str(s.clone()).map_err(|e| syn::Error::new_spanned(s, e)),
                _ => Ok(quote! { #value }),
            },
            _ => Ok(quote! { #value }),
        }
    }
}

fn map_component_type(name: Ident) -> Ident {
    let name_str = name.to_string();
    let pascal_case = match name_str.as_str() {
        "button" => "Button",
        "input" => "Input",
        "textarea" => "Textarea",
        "select" => "Select",
        "upload" => "Upload",
        "checkbox" => "Checkbox",
        "checkboxes" => "Checkboxes",
        "container" => "Container",
        "text" => "Text",
        "details" => "Details",
        "section" => "Section",
        "form" => "Form",
        "media" => "Media",
        "gallery" => "Gallery",
        "reference" => "Reference",
        "template" => "Template",
        _ => return name.clone(),
    };
    Ident::new(pascal_case, name.span())
}

fn map_button_style(expr: TokenStream) -> TokenStream {
    let expr_str = expr.to_string();
    let krate = common_crate();
    match expr_str.as_str() {
        "Primary" | "Secondary" | "Success" => {
            quote! { #krate::v2::types::components::interactive::ButtonStyle::#expr }
        }
        _ => expr,
    }
}
