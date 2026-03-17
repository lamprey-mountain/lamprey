use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse2, spanned::Spanned, Attribute, Fields, Ident, Item, ItemMod, ItemStruct, LitStr};

use crate::parse::{EndpointArgs, EndpointField, FieldKind};

pub fn expand(args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let args: EndpointArgs = parse2(args)?;
    let module: ItemMod = parse2(item)?;

    let mod_name = &module.ident;
    let mod_vis = &module.vis;
    let mod_attrs: Vec<_> = module
        .attrs
        .iter()
        .filter(|a| a.path().is_ident("doc"))
        .collect();

    let items = module
        .content
        .as_ref()
        .map(|(_, items)| items.as_slice())
        .unwrap_or(&[]);

    let request_struct = find_struct(items, "Request")?;
    let response_struct = find_struct(items, "Response")?;

    let req_fields = extract_fields(request_struct)?;
    let _resp_fields = extract_fields(response_struct)?;

    // Validate: at most one json field
    let json_count = req_fields
        .iter()
        .filter(|f| matches!(f.kind, FieldKind::Json))
        .count();
    if json_count > 1 {
        return Err(syn::Error::new(
            Span::call_site(),
            "Request may have at most one #[json] field",
        ));
    }

    let request_clean = build_clean_struct(request_struct.clone())?;
    let response_clean = build_clean_struct(response_struct.clone())?;
    let extract_fn = build_extract_fn(&req_fields)?;
    let meta_fn = build_meta_fn(&args)?;

    let expanded = quote! {
        #(#mod_attrs)*
        #mod_vis mod #mod_name {
            use super::*;

            #[derive(Debug)]
            #request_clean

            #[derive(Debug)]
            #response_clean

            #extract_fn
            #meta_fn
        }
    };

    Ok(expanded)
}

// ---------------------------------------------------------------------------
// __extract
// ---------------------------------------------------------------------------

fn build_extract_fn(fields: &[EndpointField]) -> syn::Result<TokenStream> {
    let path_fields: Vec<_> = fields
        .iter()
        .filter(|f| matches!(f.kind, FieldKind::Path(_)))
        .collect();
    let query_fields: Vec<_> = fields
        .iter()
        .filter(|f| matches!(f.kind, FieldKind::Query(_)))
        .collect();
    let header_fields: Vec<_> = fields
        .iter()
        .filter(|f| matches!(f.kind, FieldKind::Header(_)))
        .collect();
    let json_field: Option<&EndpointField> =
        fields.iter().find(|f| matches!(f.kind, FieldKind::Json));

    // --- path extraction ---
    // axum Path extractor uses a tuple for multiple params, single value for one
    let path_extraction = if path_fields.is_empty() {
        quote! {}
    } else {
        let path_idents: Vec<_> = path_fields.iter().map(|f| &f.ident).collect();
        let path_tys: Vec<_> = path_fields.iter().map(|f| &f.ty).collect();
        if path_fields.len() == 1 {
            quote! {
                let ::axum::extract::Path(#(#path_idents)*) =
                    ::axum::extract::Path::<#(#path_tys)*>::from_request_parts(&mut parts, state)
                        .await
                        .map_err(|e| e.into_response())?;
            }
        } else {
            quote! {
                let ::axum::extract::Path((#(#path_idents,)*)) =
                    ::axum::extract::Path::<(#(#path_tys,)*)>::from_request_parts(&mut parts, state)
                        .await
                        .map_err(|e| e.into_response())?;
            }
        }
    };

    // --- query extraction ---
    // Build an intermediate struct for serde query deserialization
    let query_extraction = if query_fields.is_empty() {
        quote! {}
    } else {
        let q_idents: Vec<_> = query_fields.iter().map(|f| &f.ident).collect();
        let q_tys: Vec<_> = query_fields.iter().map(|f| &f.ty).collect();
        let q_renames: Vec<_> = query_fields
            .iter()
            .map(|f| match &f.kind {
                FieldKind::Query(Some(name)) => quote! { #[serde(rename = #name)] },
                _ => quote! {},
            })
            .collect();
        quote! {
            #[derive(::serde::Deserialize)]
            struct __QueryParams {
                #(#q_renames #q_idents: #q_tys,)*
            }
            let ::axum::extract::Query(__qp) =
                ::axum::extract::Query::<__QueryParams>::from_request_parts(&mut parts, state)
                    .await
                    .map_err(|e| e.into_response())?;
            #(let #q_idents = __qp.#q_idents;)*
        }
    };

    // --- header extraction ---
    let header_extraction = if header_fields.is_empty() {
        quote! {}
    } else {
        let stmts: Vec<_> = header_fields
            .iter()
            .map(|f| {
                let ident = &f.ident;
                let ty = &f.ty;
                let header_name = match &f.kind {
                    FieldKind::Header(Some(n)) => n.clone(),
                    _ => ident.to_string().replace('_', "-"),
                };
                quote! {
                    let #ident: #ty = parts
                        .headers
                        .get(#header_name)
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.parse().ok())
                        .ok_or_else(|| {
                            ::axum::response::Response::builder()
                                .status(::axum::http::StatusCode::BAD_REQUEST)
                                .body(::axum::body::Body::from(
                                    format!("missing or invalid header: {}", #header_name)
                                ))
                                .unwrap()
                        })?;
                }
            })
            .collect();
        quote! { #(#stmts)* }
    };

    // --- json extraction ---
    // replace the json_extraction block:
    let json_extraction = if let Some(f) = json_field {
        let ident = &f.ident;
        let ty = &f.ty;
        quote! {
            let req = ::axum::http::Request::from_parts(parts, body);
            let ::axum::extract::Json(#ident) =
                ::axum::extract::Json::<#ty>::from_request(req, &state)
                    .await
                    .map_err(|e| e.into_response())?;
        }
    } else {
        quote! { let _ = body; }
    };

    // --- assemble Request struct ---
    let all_idents: Vec<_> = fields.iter().map(|f| &f.ident).collect();

    Ok(quote! {
        pub async fn __extract(
            req: ::axum::extract::Request,
        ) -> Result<Request, ::axum::response::Response> {
            use ::axum::extract::{FromRequest, FromRequestParts};
            use ::axum::response::IntoResponse;

            let (mut parts, body) = req.into_parts();
            let state = ();

            #path_extraction
            #query_extraction
            #header_extraction
            #json_extraction

            Ok(Request {
                #(#all_idents,)*
            })
        }
    })
}

// ---------------------------------------------------------------------------
// __meta
// ---------------------------------------------------------------------------

fn build_meta_fn(args: &EndpointArgs) -> syn::Result<TokenStream> {
    let method = &args.method;
    let path = &args.path;

    let mut tags: Vec<LitStr> = args.tags.clone();
    for s in &args.scopes {
        tags.push(LitStr::new(&format!("badge.scope.{}", s.value()), s.span()));
    }
    for p in &args.permissions {
        tags.push(LitStr::new(&format!("badge.perm.{}", p.value()), p.span()));
    }
    for p in &args.permissions_optional {
        tags.push(LitStr::new(
            &format!("badge.perm-opt.{}", p.value()),
            p.span(),
        ));
    }

    Ok(quote! {
        pub fn __meta() -> (&'static str, &'static str, &'static [&'static str]) {
            (&[#(#tags,)*], #method, #path)
        }
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn find_struct<'a>(items: &'a [Item], name: &str) -> syn::Result<&'a ItemStruct> {
    items
        .iter()
        .find_map(|item| {
            if let Item::Struct(s) = item {
                if s.ident == name {
                    return Some(s);
                }
            }
            None
        })
        .ok_or_else(|| {
            syn::Error::new(
                Span::call_site(),
                format!("endpoint module must contain a `{name}` struct"),
            )
        })
}

fn extract_fields(s: &ItemStruct) -> syn::Result<Vec<EndpointField>> {
    let fields = match &s.fields {
        Fields::Named(f) => &f.named,
        _ => {
            return Err(syn::Error::new(
                s.span(),
                "Request/Response must have named fields",
            ))
        }
    };

    fields
        .iter()
        .map(|f| {
            let ident = f.ident.clone().unwrap();
            let ty = f.ty.clone();
            let kind = extract_field_kind(&f.attrs, &ident)?;
            let doc = f
                .attrs
                .iter()
                .filter(|a| a.path().is_ident("doc"))
                .cloned()
                .collect();
            Ok(EndpointField {
                kind,
                ident,
                ty,
                doc,
            })
        })
        .collect()
}

fn extract_field_kind(attrs: &[Attribute], ident: &Ident) -> syn::Result<FieldKind> {
    for attr in attrs {
        let path = attr.path();
        if path.is_ident("path") {
            return Ok(FieldKind::Path(try_parse_str_arg(attr)));
        }
        if path.is_ident("query") {
            return Ok(FieldKind::Query(try_parse_str_arg(attr)));
        }
        if path.is_ident("header") {
            return Ok(FieldKind::Header(try_parse_str_arg(attr)));
        }
        if path.is_ident("json") {
            return Ok(FieldKind::Json);
        }
    }
    Err(syn::Error::new(
        ident.span(),
        "field must have one of: #[path], #[query], #[header], #[json]",
    ))
}

fn try_parse_str_arg(attr: &Attribute) -> Option<String> {
    attr.parse_args::<LitStr>().ok().map(|s| s.value())
}

fn build_clean_struct(mut original: ItemStruct) -> syn::Result<TokenStream> {
    for field in &mut original.fields {
        field.attrs.retain(|attr| {
            !attr.path().is_ident("path")
                && !attr.path().is_ident("query")
                && !attr.path().is_ident("header")
                && !attr.path().is_ident("json")
        });
    }
    Ok(quote! { #original })
}
