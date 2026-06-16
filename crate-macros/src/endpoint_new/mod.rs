//! work in progress rewrite for the endpoint macro

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{Attribute, Fields, Ident, Item, ItemMod, ItemStruct, LitStr, parse2, spanned::Spanned};

use crate::endpoint_new::parse::{
    EndpointArgs, EndpointDocs, EndpointField, EndpointModule, FieldKind,
};

mod parse;

pub fn expand(args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let args: EndpointArgs = parse2(args)?;
    let module: EndpointModule = parse2(item)?;

    module.validate()?;

    let mod_name = &module.module.ident;
    let mod_vis = &module.module.vis;
    let mod_attrs: Vec<_> = module
        .module
        .attrs
        .iter()
        .filter(|a| a.path().is_ident("doc"))
        .collect();

    let request_clean = build_clean_struct(module.request.clone())?;
    let response_clean = build_clean_struct(module.response.clone())?;

    let req_fields = extract_fields(&module.request)?;
    let resp_fields = extract_fields(&module.response)?;

    let extract_request_fn = build_extract_request_fn(&args, &req_fields)?;
    let encode_request_fn = build_encode_request_fn(&args, &req_fields)?;
    let encode_response_fn = build_encode_response_fn(&args, &resp_fields)?;
    let extract_response_fn = build_extract_response_fn(&args, &resp_fields)?;

    let meta_fn = build_metadata_fn(&args, &module.module)?;

    // Preserve all original items except Request/Response structs
    let items = module
        .module
        .content
        .as_ref()
        .map(|(_, items)| items.as_slice())
        .unwrap_or(&[]);
    let preserved_items: Vec<_> = items
        .iter()
        .filter(|item| {
            if let Item::Struct(s) = item {
                s.ident != "Request" && s.ident != "Response"
            } else {
                true
            }
        })
        .collect();

    let expanded = quote! {
        #(#mod_attrs)*
        #mod_vis mod #mod_name {
            use super::*;

            pub struct Endpoint;

            #[derive(Debug, Clone)]
            #request_clean

            #[derive(Debug, Clone)]
            #response_clean

            impl crate::util::routes::Endpoint for Endpoint {
                type Request = Request;
                type Response = Response;

                #meta_fn
            }

            impl crate::util::routes::Request for Request {
                #encode_request_fn
                #extract_request_fn
            }

            impl crate::util::routes::Response for Response {
                #encode_response_fn
                #extract_response_fn
            }

            #(#preserved_items)*
        }
    };

    Ok(expanded)
}

fn build_encode_response_fn(
    args: &EndpointArgs,
    fields: &[EndpointField],
) -> syn::Result<TokenStream> {
    let status_code = if let Some(spec) = args.responses.first() {
        let s = &spec.status;
        quote! { ::http::StatusCode::from_u16(#s).unwrap_or(::http::StatusCode::OK) }
    } else {
        quote! { ::http::StatusCode::OK }
    };

    let json_field = fields.iter().find(|f| matches!(f.kind, FieldKind::Json));

    if let Some(json_field) = json_field {
        let ident = &json_field.ident;
        Ok(quote! {
            fn encode(self) -> ::http::Response<::bytes::Bytes> {
                let json = ::serde_json::to_string(&self.#ident)
                    .unwrap_or_else(|e| format!("{{\"error\": \"serialization failed: {}\"}}", e));
                ::http::Response::builder()
                    .status(#status_code)
                    .header(::http::header::CONTENT_TYPE, "application/json")
                    .body(::bytes::Bytes::from(json))
                    .unwrap()
            }
        })
    } else {
        Ok(quote! {
            fn encode(self) -> ::http::Response<::bytes::Bytes> {
                ::http::Response::builder()
                    .status(#status_code)
                    .body(::bytes::Bytes::new())
                    .unwrap()
            }
        })
    }
}

fn build_extract_response_fn(
    args: &EndpointArgs,
    fields: &[EndpointField],
) -> syn::Result<TokenStream> {
    let status_check = if !args.responses.is_empty() {
        let codes: Vec<_> = args.responses.iter().map(|r| &r.status).collect();
        quote! {
            let allowed = [#(#codes),*];
            if !allowed.contains(&status.as_u16()) {
                return Err(resp);
            }
        }
    } else {
        quote! {
            if !status.is_success() {
                return Err(resp);
            }
        }
    };

    let json_field = fields.iter().find(|f| matches!(f.kind, FieldKind::Json));

    if let Some(json_field) = json_field {
        let ident = &json_field.ident;
        let ty = &json_field.ty;

        Ok(quote! {
            fn extract(resp: ::http::Response<::bytes::Bytes>) -> ::core::result::Result<Self, ::http::Response<::bytes::Bytes>> {
                let status = resp.status();
                #status_check
                let (_parts, body) = resp.into_parts();
                let #ident: #ty = ::serde_json::from_slice(&body)
                    .map_err(|_| {
                        // FIXME: proper error handling
                        ::http::Response::builder()
                            .status(::http::StatusCode::INTERNAL_SERVER_ERROR)
                            .body(::bytes::Bytes::from("failed to parse response json"))
                            .unwrap()
                    })?;
                Ok(Response { #ident })
            }
        })
    } else {
        Ok(quote! {
            fn extract(resp: ::http::Response<::bytes::Bytes>) -> ::core::result::Result<Self, ::http::Response<::bytes::Bytes>> {
                let status = resp.status();
                #status_check
                Ok(Response {})
            }
        })
    }
}

fn build_encode_request_fn(
    args: &EndpointArgs,
    fields: &[EndpointField],
) -> syn::Result<TokenStream> {
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
    let json_field = fields.iter().find(|f| matches!(f.kind, FieldKind::Json));
    let form_field = fields.iter().find(|f| matches!(f.kind, FieldKind::Form));
    let body_field = fields.iter().find(|f| matches!(f.kind, FieldKind::Body));

    let method_str = args.method.value();
    let method_ident = Ident::new(&method_str, args.method.span());
    let method = quote! { ::http::Method::#method_ident };

    // PERF: use format!() instead of string.replace
    let path_template = args.path.value();
    let path_build = if path_fields.is_empty() {
        quote! {
            let mut url = String::from(#path_template);
        }
    } else {
        let mut replacements = Vec::new();
        for f in &path_fields {
            let param_name = f.ident.to_string();
            let placeholder = format!("{{{}}}", param_name);
            let ident = &f.ident;
            replacements.push(quote! {
                url = url.replace(#placeholder, &::std::format!("{}", self.#ident));
            });

            if let FieldKind::Path(Some(rename)) = &f.kind {
                let placeholder_renamed = format!("{{{}}}", rename);
                replacements.push(quote! {
                    url = url.replace(#placeholder_renamed, &::std::format!("{}", self.#ident));
                });
            }
        }
        quote! {
            let mut url = String::from(#path_template);
            #(#replacements)*
        }
    };

    let query_build = if query_fields.is_empty() {
        quote! {}
    } else {
        let parts: Vec<_> = query_fields
            .iter()
            .map(|f| {
                let ident = &f.ident;
                quote! {
                    let qs = ::serde_urlencoded::to_string(&self.#ident)
                        .unwrap_or_else(|e| panic!("query serialization failed: {}", e));
                    if !qs.is_empty() {
                        query_parts.push(qs);
                    }
                }
            })
            .collect();
        quote! {
            let mut query_parts: Vec<String> = Vec::new();
            #(#parts)*
            if !query_parts.is_empty() {
                url.push_str("?");
                url.push_str(&query_parts.join("&"));
            }
        }
    };

    let header_build = if header_fields.is_empty() {
        quote! {}
    } else {
        let stmts: Vec<_> = header_fields
            .iter()
            .map(|f| {
                let ident = &f.ident;
                let header_name = match &f.kind {
                    FieldKind::Header(Some(n)) => n.clone(),
                    _ => f.ident.to_string().replace('_', "-"),
                };
                let is_option = matches!(&f.ty, syn::Type::Path(tp) if tp.path.segments.last().map(|s| s.ident == "Option").unwrap_or(false));
                if is_option {
                    quote! {
                        if let Some(ref val) = self.#ident {
                            req_builder = req_builder.header(#header_name, ::std::format!("{}", val));
                        }
                    }
                } else {
                    quote! {
                        req_builder = req_builder.header(#header_name, ::std::format!("{}", self.#ident));
                    }
                }
            })
            .collect();
        quote! { #(#stmts)* }
    };

    let body_build = if let Some(f) = json_field {
        let ident = &f.ident;
        quote! {
            let body: ::bytes::Bytes = ::serde_json::to_vec(&self.#ident)
                .unwrap_or_else(|e| panic!("json serialization failed: {}", e))
                .into();
            req_builder = req_builder.header(::http::header::CONTENT_TYPE, "application/json");
        }
    } else if let Some(f) = form_field {
        let ident = &f.ident;
        quote! {
            let body: ::bytes::Bytes = ::serde_urlencoded::to_string(&self.#ident)
                .unwrap_or_else(|e| panic!("form serialization failed: {}", e))
                .into_bytes()
                .into();
            req_builder = req_builder.header(::http::header::CONTENT_TYPE, "application/x-www-form-urlencoded");
        }
    } else if let Some(f) = body_field {
        let ident = &f.ident;
        quote! {
            let body: ::bytes::Bytes = self.#ident;
        }
    } else {
        quote! {
            let body = ::bytes::Bytes::new();
        }
    };

    Ok(quote! {
        fn encode(self) -> ::http::Request<::bytes::Bytes> {
            #path_build
            #query_build

            let mut req_builder = ::http::Request::builder()
                .method(#method)
                .uri(&url);

            #header_build
            #body_build

            req_builder.body(body).unwrap()
        }
    })
}

fn build_extract_request_fn(
    args: &EndpointArgs,
    fields: &[EndpointField],
) -> syn::Result<TokenStream> {
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
    let json_field = fields.iter().find(|f| matches!(f.kind, FieldKind::Json));
    let form_field = fields.iter().find(|f| matches!(f.kind, FieldKind::Form));
    let body_field = fields.iter().find(|f| matches!(f.kind, FieldKind::Body));

    let method_str = args.method.value();
    let method_ident = Ident::new(&method_str, args.method.span());

    let path_template = args.path.value();
    let (match_arm_pattern, extract_bindings) = build_path_match_pattern(&path_template)?;

    let path_extraction = if path_fields.is_empty() {
        quote! {}
    } else {
        let conversions: Vec<TokenStream> = path_fields
            .iter()
            .map(|f| {
                let ident = &f.ident;
                let ty = &f.ty;
                let raw_name = format_ident!("{}_raw", f.ident);
                quote! {
                    let #ident: #ty = crate::v1::routes::PathParam::from_path_param(#raw_name)
                        .map_err(|_| original_req.clone())?;
                }
            })
            .collect();
        quote! {
            let decoded_path = ::percent_encoding::percent_decode_str(path)
                .decode_utf8()
                .unwrap_or_else(|_| path.into());
            let segments = decoded_path.split('/').collect::<Vec<_>>();
            let (#extract_bindings) = match segments.as_slice() {
                #match_arm_pattern => (#extract_bindings),
                _ => return Err(original_req),
            };
            #(#conversions)*
        }
    };

    let query_extraction = if query_fields.is_empty() {
        quote! {}
    } else {
        let mut stmts = Vec::new();
        let mut named_idents = Vec::new();
        let mut named_tys = Vec::new();
        let mut named_renames = Vec::new();

        for f in &query_fields {
            match &f.kind {
                FieldKind::Query(Some(name)) => {
                    named_idents.push(&f.ident);
                    named_tys.push(&f.ty);
                    named_renames.push(quote! { #[serde(rename = #name)] });
                }
                FieldKind::Query(None) => {
                    let ident = &f.ident;
                    let ty = &f.ty;
                    stmts.push(quote! {
                        let #ident: #ty = ::serde_urlencoded::from_str(query_str)
                            .map_err(|_| original_req.clone())?;
                    });
                }
                _ => {}
            }
        }

        let named_extraction = if named_idents.is_empty() {
            quote! {}
        } else {
            quote! {
                #[derive(::serde::Deserialize)]
                struct __QueryParams {
                    #(#named_renames #named_idents: #named_tys,)*
                }
                let __qp: __QueryParams = ::serde_urlencoded::from_str(query_str)
                    .map_err(|_| original_req.clone())?;
                #(let #named_idents = __qp.#named_idents;)*
            }
        };

        quote! {
            #named_extraction
            #(#stmts)*
        }
    };

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
                let is_option = matches!(ty, syn::Type::Path(tp) if tp.path.segments.last().map(|s| s.ident == "Option").unwrap_or(false));
                if is_option {
                    quote! {
                        let #ident: #ty = parts
                            .headers
                            .get(#header_name)
                            .and_then(|v| v.to_str().ok())
                            .and_then(|v| v.parse().ok());
                    }
                } else {
                    quote! {
                        let #ident: #ty = parts
                            .headers
                            .get(#header_name)
                            .and_then(|v| v.to_str().ok())
                            .and_then(|v| v.parse().ok())
                            .ok_or_else(|| original_req.clone())?;
                    }
                }
            })
            .collect();
        quote! { #(#stmts)* }
    };

    let body_extraction = if let Some(f) = json_field {
        let ident = &f.ident;
        let ty = &f.ty;
        quote! {
            let #ident: #ty = ::serde_json::from_slice(&body)
                .map_err(|_| original_req.clone())?;
        }
    } else if let Some(f) = form_field {
        let ident = &f.ident;
        let ty = &f.ty;
        quote! {
            let #ident: #ty = ::serde_urlencoded::from_str::<#ty>(
                &std::str::from_utf8(&body).map_err(|_| original_req.clone())?
            ).map_err(|_| original_req.clone())?;
        }
    } else if let Some(f) = body_field {
        let ident = &f.ident;
        quote! {
            let #ident = body;
        }
    } else {
        quote! {}
    };

    let all_idents: Vec<_> = fields.iter().map(|f| &f.ident).collect();

    Ok(quote! {
        fn extract(req: ::http::Request<::bytes::Bytes>) -> ::core::result::Result<Self, ::http::Request<::bytes::Bytes>> {
            if req.method() != ::http::Method::#method_ident {
                return Err(req);
            }

            let original_req = req;
            let (parts, body) = original_req.clone().into_parts();

            let path = parts.uri.path();
            let query_str = parts.uri.query().unwrap_or("");

            #path_extraction
            #query_extraction
            #header_extraction
            #body_extraction

            Ok(Request {
                #(#all_idents,)*
            })
        }
    })
}

fn build_metadata_fn(args: &EndpointArgs, module: &ItemMod) -> syn::Result<TokenStream> {
    let operation_id = LitStr::new(&module.ident.to_string(), module.ident.span());

    let method_str = args.method.value();
    let method_pascal = match method_str.as_str() {
        "GET" => "Get",
        "POST" => "Post",
        "PUT" => "Put",
        "PATCH" => "Patch",
        "DELETE" => "Delete",
        "HEAD" => "Head",
        _ => method_str.as_str(),
    };
    let method_ident = Ident::new(method_pascal, Span::call_site());
    let path = &args.path;

    let mut tags_full: Vec<LitStr> = args.tags.clone();
    for s in &args.scopes {
        tags_full.push(LitStr::new(&format!("badge.scope.{}", s), s.span()));
    }
    for p in &args.permissions {
        tags_full.push(LitStr::new(&format!("badge.perm.{}", p), p.span()));
    }
    for p in &args.permissions_optional {
        tags_full.push(LitStr::new(&format!("badge.perm-opt.{}", p), p.span()));
    }
    for p in &args.permissions_server {
        tags_full.push(LitStr::new(&format!("badge.server-perm.{}", p), p.span()));
    }
    for p in &args.permissions_server_optional {
        tags_full.push(LitStr::new(
            &format!("badge.server-perm-opt.{}", p),
            p.span(),
        ));
    }
    for e in &args.audit_log_events {
        tags_full.push(LitStr::new(
            &format!("badge.audit-log.{}", e.value()),
            e.span(),
        ));
    }

    let user_tags = &args.tags;
    let scopes = &args.scopes;
    let permissions = &args.permissions;
    let permissions_optional = &args.permissions_optional;
    let permissions_server = &args.permissions_server;
    let permissions_server_optional = &args.permissions_server_optional;
    let audit_log_events = &args.audit_log_events;

    let (summary_text, description_text) = parse_doc_attrs(&module.attrs);
    let summary = LitStr::new(&summary_text, Span::call_site());
    let description = if let Some(desc) = description_text {
        quote! { Some(#desc) }
    } else {
        quote! { None }
    };

    Ok(quote! {
        fn metadata() -> crate::util::routes::Metadata {
            crate::util::routes::Metadata {
                operation_id: #operation_id,
                summary: #summary,
                description: #description,
                method: crate::v1::routes::EndpointMethod::#method_ident,
                path: #path,
                tags: &[#(#user_tags,)*],
                tags_full: &[#(#tags_full,)*],
                scopes: &[#(crate::v1::types::oauth::Scope::#scopes,)*],
                permissions: &[#(crate::v1::types::Permission::#permissions,)*],
                permissions_optional: &[#(crate::v1::types::Permission::#permissions_optional,)*],
                permissions_server: &[#(crate::v1::types::Permission::#permissions_server,)*],
                permissions_server_optional: &[#(crate::v1::types::Permission::#permissions_server_optional,)*],
                audit_log_events: &[#(#audit_log_events,)*],
            }
        }
    })
}

// NOTE: this works, but isn't particularily robust
fn build_path_match_pattern(template: &str) -> syn::Result<(TokenStream, TokenStream)> {
    let mut pattern_items: Vec<TokenStream> = Vec::new();
    let mut bindings: Vec<TokenStream> = Vec::new();

    for segment in template.split('/') {
        if let Some(param_name) = segment.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
            let raw_name = format_ident!("{}_raw", param_name);
            pattern_items.push(quote! { #raw_name });
            bindings.push(quote! { #raw_name });
        } else {
            pattern_items.push(quote! { #segment });
        }
    }

    let pattern = quote! { [#(#pattern_items),*] };
    let bindings = quote! { (#(#bindings),*) };

    Ok((pattern, bindings))
}

fn parse_doc_attrs(attrs: &[Attribute]) -> (String, Option<String>) {
    let mut doc_lines: Vec<String> = Vec::new();

    for attr in attrs {
        if let syn::Meta::NameValue(nv) = &attr.meta {
            if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(lit),
                ..
            }) = &nv.value
            {
                let value = lit.value();
                let trimmed = value.trim();
                doc_lines.push(trimmed.to_string());
            }
        }
    }

    let summary = doc_lines
        .first()
        .map(|s| s.clone())
        .unwrap_or_else(|| "".to_string());
    let description = if doc_lines.len() > 1 {
        let rest = &doc_lines[1..];
        let start = rest
            .iter()
            .position(|l| !l.is_empty())
            .unwrap_or(rest.len());
        let desc = rest[start..].join("\n");

        if desc.is_empty() { None } else { Some(desc) }
    } else {
        None
    };

    (summary, description)
}

/// remove all macro-specific attributes from a struct
fn build_clean_struct(mut original: ItemStruct) -> syn::Result<TokenStream> {
    for field in &mut original.fields {
        field.attrs.retain(|attr| {
            !attr.path().is_ident("path")
                && !attr.path().is_ident("query")
                && !attr.path().is_ident("header")
                && !attr.path().is_ident("json")
                && !attr.path().is_ident("form")
                && !attr.path().is_ident("body")
        });
    }
    Ok(quote! { #original })
}

/// extract endpoint fields from a struct
fn extract_fields(s: &ItemStruct) -> syn::Result<Vec<EndpointField>> {
    let fields = match &s.fields {
        Fields::Named(f) => &f.named,
        _ => {
            return Err(syn::Error::new(
                s.span(),
                "Request/Response must have named fields",
            ));
        }
    };

    fields
        .iter()
        .map(|f| {
            let ident = f.ident.clone().unwrap();
            let ty = f.ty.clone();
            let kind = extract_field_kind(&f.attrs, &ident)?;
            let (summary, description) = parse_doc_attrs(&f.attrs);
            let doc = EndpointDocs {
                summary,
                description,
            };
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
            let rename = if matches!(attr.meta, syn::Meta::Path(_)) {
                None
            } else {
                try_parse_rename_arg(attr)?
            };
            return Ok(FieldKind::Path(rename));
        }
        if path.is_ident("query") {
            let rename = if matches!(attr.meta, syn::Meta::Path(_)) {
                None
            } else {
                try_parse_rename_arg(attr)?
            };
            return Ok(FieldKind::Query(rename));
        }
        if path.is_ident("header") {
            let rename = if matches!(attr.meta, syn::Meta::Path(_)) {
                None
            } else {
                try_parse_rename_arg(attr)?
            };
            return Ok(FieldKind::Header(rename));
        }
        if path.is_ident("json") {
            return Ok(FieldKind::Json);
        }
        if path.is_ident("form") {
            return Ok(FieldKind::Form);
        }
        if path.is_ident("body") {
            return Ok(FieldKind::Body);
        }
    }
    Err(syn::Error::new(
        ident.span(),
        "field must have one of: #[path], #[query], #[header], #[json], #[form], #[body]",
    ))
}

fn try_parse_rename_arg(attr: &Attribute) -> syn::Result<Option<String>> {
    let meta = attr.parse_args::<syn::Meta>().map_err(|_| {
        syn::Error::new(
            attr.span(),
            "attribute must use the form #[attr(rename = \"...\")]",
        )
    })?;
    if let syn::Meta::NameValue(nv) = meta {
        if nv.path.is_ident("rename") {
            match nv.value {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(lit),
                    ..
                }) => return Ok(Some(lit.value())),
                syn::Expr::Path(_) => {
                    return Err(syn::Error::new(
                        nv.value.span(),
                        "rename value must be a string literal, e.g., rename = \"...\"",
                    ));
                }
                _ => {
                    return Err(syn::Error::new(
                        nv.value.span(),
                        "rename value must be a string literal",
                    ));
                }
            }
        } else {
            return Err(syn::Error::new(
                nv.path.span(),
                "unknown attribute argument, expected `rename`",
            ));
        }
    }
    Err(syn::Error::new(
        attr.span(),
        "attribute must use the form #[attr(rename = \"...\")]",
    ))
}
