#![allow(dead_code)] // TEMP: suppress errors during initial dev

use proc_macro2::TokenStream;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Attribute, Ident, LitStr, Token, Type,
};

pub struct EndpointArgs {
    pub method: LitStr,
    pub path: LitStr,
    pub tags: Vec<LitStr>,
    pub scopes: Vec<Ident>,
    pub permissions: Vec<Ident>,
    pub permissions_optional: Vec<Ident>,
    pub permissions_server: Vec<Ident>,
    pub permissions_server_optional: Vec<Ident>,
    pub responses: Vec<ResponseSpec>,
}

pub struct ResponseSpec {
    pub status: syn::LitInt,
    pub description: Option<LitStr>,
    pub body: Option<Type>,
}

#[derive(Clone)]
pub enum FieldKind {
    Path(Option<String>),
    Query(Option<String>),
    Header(Option<String>),
    Json,
    Form,
}

#[derive(Clone)]
pub struct EndpointField {
    pub kind: FieldKind,
    pub ident: syn::Ident,
    pub ty: Type,
    pub doc: Vec<Attribute>,
}

impl Parse for EndpointArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut method = None;
        let mut path = None;
        let mut tags = vec![];
        let mut scopes = vec![];
        let mut permissions = vec![];
        let mut permissions_optional = vec![];
        let mut permissions_server = vec![];
        let mut permissions_server_optional = vec![];
        let mut responses = vec![];

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            let key_str = key.to_string();

            // Bare HTTP method keyword
            if matches!(
                key_str.as_str(),
                "get" | "post" | "put" | "patch" | "delete" | "head" | "options"
            ) {
                method = Some(LitStr::new(&key_str.to_uppercase(), key.span()));
                if input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                }
                continue;
            }

            // Paren-style args (no `=`)
            if matches!(key_str.as_str(), "response" | "errors") {
                match key_str.as_str() {
                    "response" => responses.push(parse_response_parens(input)?),
                    "errors" => parse_ignore_parens(input)?,
                    _ => unreachable!(),
                }
                if input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                }
                continue;
            }

            input.parse::<Token![=]>()?;

            match key_str.as_str() {
                "method" => method = Some(input.parse::<LitStr>()?),
                "path" => path = Some(input.parse::<LitStr>()?),
                "tags" => tags = parse_str_array(input)?,
                "scopes" => scopes = parse_ident_array(input)?,
                "permissions" => permissions = parse_ident_array(input)?,
                "permissions_optional" => permissions_optional = parse_ident_array(input)?,
                "permissions_server" => permissions_server = parse_ident_array(input)?,
                "permissions_server_optional" => {
                    permissions_server_optional = parse_ident_array(input)?
                }
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("unknown endpoint arg `{other}`"),
                    ))
                }
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(EndpointArgs {
            method: method.ok_or_else(|| input.error("missing method (e.g. `get` or `post`)"))?,
            path: path.ok_or_else(|| input.error("missing `path`"))?,
            tags,
            scopes,
            permissions,
            permissions_optional,
            permissions_server,
            permissions_server_optional,
            responses,
        })
    }
}

fn parse_str_array(input: ParseStream) -> syn::Result<Vec<LitStr>> {
    let content;
    syn::bracketed!(content in input);
    let items: Punctuated<LitStr, Token![,]> =
        content.parse_terminated(|i| i.parse::<LitStr>(), Token![,])?;
    Ok(items.into_iter().collect())
}

fn parse_ident_array(input: ParseStream) -> syn::Result<Vec<Ident>> {
    let content;
    syn::bracketed!(content in input);
    let items: Punctuated<Ident, Token![,]> =
        content.parse_terminated(|i| i.parse::<Ident>(), Token![,])?;
    Ok(items.into_iter().collect())
}

fn parse_response_parens(input: ParseStream) -> syn::Result<ResponseSpec> {
    let content;
    syn::parenthesized!(content in input);

    // Accept either an integer literal (200) or an ident (OK, CREATED)
    let status: syn::LitInt = if content.peek(syn::LitInt) {
        content.parse()?
    } else {
        let ident: Ident = content.parse()?;
        // Convert to the numeric value axum/http uses, or just re-emit as string
        // Store as a fake LitInt by stringifying — caller uses it in quote! anyway
        syn::LitInt::new(
            match ident.to_string().as_str() {
                "OK" => "200",
                "CREATED" => "201",
                "ACCEPTED" => "202",
                "NO_CONTENT" => "204",
                "NOT_MODIFIED" => "304",
                "BAD_REQUEST" => "400",
                "UNAUTHORIZED" => "401",
                "FORBIDDEN" => "403",
                "NOT_FOUND" => "404",
                "CONFLICT" => "409",
                "UNPROCESSABLE_ENTITY" => "422",
                "INTERNAL_SERVER_ERROR" => "500",
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!("unknown status code `{other}`, use a numeric literal instead"),
                    ))
                }
            },
            ident.span(),
        )
    };

    let mut description = None;
    let mut body = None;
    while content.peek(Token![,]) {
        content.parse::<Token![,]>()?;
        if content.is_empty() {
            break;
        }
        let k: Ident = content.parse()?;
        content.parse::<Token![=]>()?;
        match k.to_string().as_str() {
            "description" => description = Some(content.parse::<LitStr>()?),
            "body" => body = Some(content.parse::<Type>()?),
            other => {
                return Err(syn::Error::new(
                    k.span(),
                    format!("unknown response field `{other}`"),
                ))
            }
        }
    }
    Ok(ResponseSpec {
        status,
        description,
        body,
    })
}

fn parse_ignore_parens(input: ParseStream) -> syn::Result<()> {
    let content;
    syn::parenthesized!(content in input);
    content.parse::<TokenStream>()?;
    Ok(())
}
