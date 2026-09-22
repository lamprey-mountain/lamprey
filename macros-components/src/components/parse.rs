use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Expr, Ident, Token, braced, parenthesized,
    parse::{Parse, ParseStream},
    parse2,
    punctuated::Punctuated,
};

/// a component definition
pub struct Component {
    pub name: Ident,
    pub args: Punctuated<Arg, Token![,]>,
    pub children: Vec<Children>,
}

/// a field for a component
pub struct Arg {
    pub name: Ident,
    pub value: Expr,
}

/// the children of a component
pub struct Children {
    pub section: Option<Ident>,
    pub components: Vec<Component>,
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
