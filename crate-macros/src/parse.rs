use proc_macro2::Span;
use syn::{
    bracketed, parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Ident, LitStr, Token, Type,
};

fn parse_str_array(input: ParseStream) -> syn::Result<Vec<LitStr>> {
    let content;
    bracketed!(content in input);
    let items: Punctuated<LitStr, Token![,]> =
        content.parse_terminated(|input| input.parse(), Token![,])?;
    Ok(items.into_iter().collect())
}

fn parse_ident_array(input: ParseStream) -> syn::Result<Vec<Ident>> {
    let content;
    bracketed!(content in input);
    let items: Punctuated<Ident, Token![,]> =
        content.parse_terminated(|input| input.parse(), Token![,])?;
    Ok(items.into_iter().collect())
}

pub struct ResponseArg {
    pub status: Ident,
    pub body: Option<Type>,
    pub description: Option<LitStr>,
}

fn parse_response(input: ParseStream) -> syn::Result<ResponseArg> {
    let content;
    parenthesized!(content in input);
    let mut status = None;
    let mut body = None;
    let mut description = None;

    loop {
        if content.is_empty() {
            break;
        }
        let key: Ident = content.parse()?;
        content.parse::<Token![=]>()?;
        match key.to_string().as_str() {
            "status" => status = Some(content.parse::<Ident>()?),
            "body" => body = Some(content.parse::<Type>()?),
            "description" => description = Some(content.parse::<LitStr>()?),
            other => {
                return Err(syn::Error::new(
                    key.span(),
                    format!("unknown response key `{other}`"),
                ))
            }
        }
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }
    }
    Ok(ResponseArg {
        status: status
            .ok_or_else(|| syn::Error::new(Span::call_site(), "response missing `status`"))?,
        body,
        description,
    })
}

pub struct EndpointArgs {
    pub method: Ident,
    pub path: LitStr,
    pub tags: Vec<LitStr>,
    pub scopes: Vec<LitStr>,
    pub permissions: Vec<LitStr>,
    pub permissions_optional: Vec<LitStr>,
    pub permissions_server: Vec<LitStr>,
    pub responses: Vec<ResponseArg>,
    pub errors: Vec<Ident>,
}

impl Parse for EndpointArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let method: Ident = input.parse()?;

        let mut path: Option<LitStr> = None;
        let mut tags = vec![];
        let mut scopes = vec![];
        let mut permissions = vec![];
        let mut permissions_optional = vec![];
        let mut permissions_server = vec![];
        let mut responses = vec![];
        let mut errors = vec![];

        while !input.is_empty() {
            input.parse::<Token![,]>()?;
            if input.is_empty() {
                break;
            }

            let key: Ident = input.parse()?;
            let key_str = key.to_string();

            // `response(...)` and `errors(...)` don't use `=`
            if key_str == "response" {
                responses.push(parse_response(input)?);
                continue;
            }
            if key_str == "errors" {
                let content;
                parenthesized!(content in input);
                let items: Punctuated<Ident, Token![,]> =
                    content.parse_terminated(Ident::parse, Token![,])?;
                errors.extend(items);
                continue;
            }

            input.parse::<Token![=]>()?;
            match key_str.as_str() {
                "path" => path = Some(input.parse()?),
                "tags" => tags = parse_str_array(input)?,
                "scopes" => scopes = parse_str_array(input)?,
                "permissions" => permissions = parse_str_array(input)?,
                "permissions_optional" => permissions_optional = parse_str_array(input)?,
                "permissions_server" => permissions_server = parse_str_array(input)?,
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("unknown key `{other}`"),
                    ))
                }
            }
        }

        Ok(EndpointArgs {
            method,
            path: path.ok_or_else(|| syn::Error::new(Span::call_site(), "missing `path`"))?,
            tags,
            scopes,
            permissions,
            permissions_optional,
            permissions_server,
            responses,
            errors,
        })
    }
}

/// A field inside Request or Response, with its lamprey attribute.
#[derive(Debug)]
pub enum FieldKind {
    Path(Option<String>),   // #[path] or #[path("name")]
    Query(Option<String>),  // #[query] or #[query("name")]
    Header(Option<String>), // #[header] or #[header("name")]
    Json,                   // #[json]
}

#[derive(Debug)]
pub struct EndpointField {
    pub kind: FieldKind,
    pub ident: Ident,
    pub ty: Type,
    pub doc: Vec<syn::Attribute>, // pass-through doc comments
}
