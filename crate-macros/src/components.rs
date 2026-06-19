use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Expr, Ident, Token, braced, parenthesized,
    parse::{Parse, ParseStream},
    parse2,
    punctuated::Punctuated,
};

pub fn expand(item: TokenStream) -> syn::Result<TokenStream> {
    let parsed: Children = parse2(item)?;

    let components = parsed
        .components
        .into_iter()
        .map(Component::expand)
        .collect::<syn::Result<Vec<_>>>()?;

    let krate = common_crate();

    Ok(quote! {
        #krate::v2::types::components::types::components::Components {
            inner: vec![ #(#components),* ],
            ..::std::default::Default::default()
        }
    })
}

struct Component {
    name: Ident,
    args: Punctuated<Arg, Token![,]>,
    children: Vec<Children>,
}

struct Arg {
    name: Ident,
    value: Expr,
}

struct Children {
    section: Option<Ident>,
    components: Vec<Component>,
}

impl Parse for Component {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;

        // handle text
        match name.to_string().as_str() {
            "text" => {
                let content;
                parenthesized!(content in input);
                let expr: Expr = content.parse()?;
                let mut args = Punctuated::new();
                args.push(Arg {
                    name: format_ident!("content"),
                    value: expr,
                });
                return Ok(Component {
                    name,
                    args,
                    children: vec![],
                });
            }
            _ => {}
        };

        let content;
        parenthesized!(content in input);
        let args = content.parse_terminated(Arg::parse, Token![,])?;

        let children = if input.peek(syn::token::Brace) {
            let content;
            braced!(content in input);

            let mut c = Vec::new();
            while !content.is_empty() {
                c.push(content.parse()?);
            }
            c
        } else {
            vec![]
        };

        Ok(Component {
            name,
            args,
            children,
        })
    }
}

impl Parse for Arg {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Ident) && input.peek2(Token![:]) {
            let name: Ident = input.parse()?;
            let _: Token![:] = input.parse()?;
            let value: Expr = input.parse()?;
            Ok(Arg { name, value })
        } else {
            let name: Ident = input.parse()?;
            let value: Expr = syn::parse_quote!(#name);
            Ok(Arg { name, value })
        }
    }
}

impl Parse for Children {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let section = if input.peek(Ident) && input.peek2(Token![:]) {
            let name: Ident = input.parse()?;
            let _: Token![:] = input.parse()?;
            Some(name)
        } else {
            None
        };

        let mut c = Vec::new();
        while !input.is_empty() {
            if input.peek(Ident) && input.peek2(Token![:]) {
                break;
            }

            c.push(input.parse()?);
        }

        Ok(Children {
            section,
            components: c,
        })
    }
}

impl Component {
    /// look up an arg by name
    fn arg(&self, name: &str) -> Option<&Expr> {
        self.args
            .iter()
            .find(|a| a.name.to_string() == name)
            .map(|a| &a.value)
    }

    fn expand(self) -> syn::Result<TokenStream> {
        let ty = map_component_type(self.name.clone());
        let id = self
            .arg("id")
            .cloned()
            .unwrap_or_else(|| syn::parse_quote!(None));
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
            .map(|a| a.resolve(&ty.to_string()));

        let mut sections: Vec<(Ident, Vec<TokenStream>)> = Vec::new();
        for group in self.children {
            let section = group.section.unwrap_or_else(|| format_ident!("children"));

            let mut rendered = Vec::new();
            for component in group.components {
                rendered.push(component.expand()?);
            }

            if let Some((_, existing)) = sections.iter_mut().find(|(s, _)| *s == section) {
                existing.extend(rendered);
            } else {
                sections.push((section, rendered));
            }
        }

        let section_fields = sections.into_iter().map(|(section, items)| {
            quote! { #section: vec![ #(#items),* ] }
        });

        let krate = common_crate();

        Ok(quote! {
            #krate::v2::types::components::types::Component {
                id: #id,
                ty: #krate::v2::types::components::types::ComponentType::#ty {
                    #(#attr_fields,)*
                    #(#section_fields,)*
                },
                allow: #allow,
            }
        })
    }
}

impl Arg {
    fn resolve(&self, ty: &str) -> TokenStream {
        let value = self.value.clone();
        match (self.name.to_string().as_str(), ty) {
            ("style", "Button") => map_button_style(quote! { #value }),
            _ => quote! { #value },
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

fn common_crate() -> TokenStream {
    match crate_name("lamprey-common").unwrap() {
        FoundCrate::Itself => quote! { crate },
        FoundCrate::Name(name) => {
            let ident = format_ident!("{}", name);
            quote! { ::#ident }
        }
    }
}
