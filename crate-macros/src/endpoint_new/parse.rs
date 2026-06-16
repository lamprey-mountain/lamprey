use proc_macro2::Span;
use syn::spanned::Spanned;
use syn::{
    Ident, ItemMod, ItemStruct, LitInt, LitStr, Token, Type,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

use super::extract_fields;

pub struct EndpointModule {
    pub module: ItemMod,
    pub request: ItemStruct,
    pub response: ItemStruct,
}

// TODO: #[derive(darling::FromMeta)]
/// macro attrs for an endpoint
pub struct EndpointArgs {
    pub method: LitStr,
    pub path: LitStr,
    pub tags: Vec<LitStr>,
    pub scopes: Vec<Ident>,
    pub permissions: Vec<Ident>,
    pub permissions_optional: Vec<Ident>,
    pub permissions_server: Vec<Ident>,
    pub permissions_server_optional: Vec<Ident>,
    pub audit_log_events: Vec<LitStr>,
    pub errors: Vec<Ident>,
    pub responses: Vec<ResponseSpec>,
}

// TODO: combine fields in EndpointArgs into Vec<EndpointPermission>?
// pub struct EndpointPermission {
//     pub name: Ident,
//     pub optional: bool,
//     pub server: bool,
// }

pub struct ResponseSpec {
    pub status: LitInt,
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
    Body,
}

#[derive(Clone)]
pub struct EndpointField {
    pub kind: FieldKind,
    pub ident: Ident,
    pub ty: Type,
    pub doc: EndpointDocs,
}

#[derive(Debug, Clone)]
pub struct EndpointDocs {
    pub summary: String,
    pub description: Option<String>,
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
        let mut audit_log_events = vec![];
        let mut errors = vec![];
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

            // paren-style args (no `=`)
            if matches!(key_str.as_str(), "response" | "errors") {
                match key_str.as_str() {
                    "response" => responses.push(input.parse()?),
                    "errors" => errors.extend(parse_ident_array_parens(input)?),
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
                "scopes" => scopes = parse_ident_array_brackets(input)?,
                "permissions" => permissions = parse_ident_array_brackets(input)?,
                "permissions_optional" => permissions_optional = parse_ident_array_brackets(input)?,
                "permissions_server" => permissions_server = parse_ident_array_brackets(input)?,
                "permissions_server_optional" => {
                    permissions_server_optional = parse_ident_array_brackets(input)?
                }
                "audit_log_events" => audit_log_events = parse_str_array(input)?,
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("unknown endpoint arg `{other}`"),
                    ));
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
            audit_log_events,
            errors,
            responses,
        })
    }
}

impl Parse for ResponseSpec {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        syn::parenthesized!(content in input);

        // TODO: extract this into struct Status; impl Parse for Status
        let status: LitInt = if content.peek(LitInt) {
            // numeric http status code
            content.parse()?
        } else {
            // http status code by name
            let ident: Ident = content.parse()?;
            LitInt::new(
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
                        ));
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
                    ));
                }
            }
        }

        Ok(ResponseSpec {
            status,
            description,
            body,
        })
    }
}

/// parse a comma separated array of literal strings
fn parse_str_array(input: ParseStream) -> syn::Result<Vec<LitStr>> {
    let content;
    syn::bracketed!(content in input);
    let items: Punctuated<LitStr, Token![,]> =
        content.parse_terminated(|i| i.parse::<LitStr>(), Token![,])?;
    Ok(items.into_iter().collect())
}

/// parse a comma separated array of idents
fn parse_ident_array_brackets(input: ParseStream) -> syn::Result<Vec<Ident>> {
    let content;
    syn::bracketed!(content in input);
    let items: Punctuated<Ident, Token![,]> =
        content.parse_terminated(|i| i.parse::<Ident>(), Token![,])?;
    Ok(items.into_iter().collect())
}

/// parse a comma separated array of idents
fn parse_ident_array_parens(input: ParseStream) -> syn::Result<Vec<Ident>> {
    let content;
    syn::parenthesized!(content in input);
    let items: Punctuated<Ident, Token![,]> =
        content.parse_terminated(|i| i.parse::<Ident>(), Token![,])?;
    Ok(items.into_iter().collect())
}

impl Parse for EndpointModule {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let module: ItemMod = input.parse()?;

        let items = module
            .content
            .as_ref()
            .map(|(_, items)| items.as_slice())
            .unwrap_or(&[]);

        let request = items
            .iter()
            .find_map(|item| {
                if let syn::Item::Struct(s) = item {
                    if s.ident == "Request" {
                        return Some(s.clone());
                    }
                }
                None
            })
            .ok_or_else(|| {
                syn::Error::new(
                    module.span(),
                    "endpoint module must contain a `Request` struct",
                )
            })?;

        let response = items
            .iter()
            .find_map(|item| {
                if let syn::Item::Struct(s) = item {
                    if s.ident == "Response" {
                        return Some(s.clone());
                    }
                }
                None
            })
            .ok_or_else(|| {
                syn::Error::new(
                    module.span(),
                    "endpoint module must contain a `Response` struct",
                )
            })?;

        Ok(Self {
            module,
            request,
            response,
        })
    }
}

impl EndpointModule {
    pub fn validate(&self) -> syn::Result<()> {
        let req_fields = extract_fields(&self.request)?;
        let _resp_fields = extract_fields(&self.response)?;

        let body_count = req_fields
            .iter()
            .filter(|f| matches!(f.kind, FieldKind::Json | FieldKind::Form | FieldKind::Body))
            .count();
        if body_count > 1 {
            return Err(syn::Error::new(
                Span::call_site(),
                "Request may have at most one #[json], #[form], or #[body] field",
            ));
        }

        Ok(())
    }
}
