use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
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

    // Validate: at most one json/form field
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

    let form_count = req_fields
        .iter()
        .filter(|f| matches!(f.kind, FieldKind::Form))
        .count();
    if form_count > 1 {
        return Err(syn::Error::new(
            Span::call_site(),
            "Request may have at most one #[form] field",
        ));
    }

    if json_count > 0 && form_count > 0 {
        return Err(syn::Error::new(
            Span::call_site(),
            "Request cannot have both #[json] and #[form] fields",
        ));
    }

    let body_count = req_fields
        .iter()
        .filter(|f| matches!(f.kind, FieldKind::Body))
        .count();
    if body_count > 1 {
        return Err(syn::Error::new(
            Span::call_site(),
            "Request may have at most one #[body] field",
        ));
    }

    if body_count > 0 && (json_count > 0 || form_count > 0) {
        return Err(syn::Error::new(
            Span::call_site(),
            "Request cannot have #[body] field with #[json] or #[form] fields",
        ));
    }

    let request_clean = build_clean_struct(request_struct.clone())?;
    let response_clean = build_clean_struct(response_struct.clone())?;
    let extract_fn = build_extract_fn(&req_fields, &args.path)?;
    let encode_fn = build_encode_fn(response_struct)?;
    let meta_fn = build_meta_fn(&args, &mod_attrs)?;

    // Preserve all original items except Request/Response structs
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

            #[derive(Debug)]
            #request_clean

            #[derive(Debug)]
            #response_clean

            #extract_fn
            #encode_fn
            #meta_fn

            #(#preserved_items)*
        }
    };

    Ok(expanded)
}

// ---------------------------------------------------------------------------
// __extract
// ---------------------------------------------------------------------------

fn build_extract_fn(fields: &[EndpointField], path: &LitStr) -> syn::Result<TokenStream> {
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
    let form_field: Option<&EndpointField> =
        fields.iter().find(|f| matches!(f.kind, FieldKind::Form));
    let body_field: Option<&EndpointField> =
        fields.iter().find(|f| matches!(f.kind, FieldKind::Body));

    // Parse path template at compile time to build match pattern
    let path_template = path.value();
    let (match_arm_pattern, extract_bindings) = build_path_match_pattern(&path_template)?;

    // Build path extraction with match statement
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
                    let #ident: #ty = crate::v1::routes::PathParam::from_path_param(#raw_name)?;
                }
            })
            .collect();
        quote! {
            let decoded_path = percent_encoding::percent_decode_str(path)
                .decode_utf8()
                .unwrap_or_else(|_| path.into());
            let segments = decoded_path.split('/').collect::<Vec<_>>();
            let (#extract_bindings) = match segments.as_slice() {
                #match_arm_pattern => (#extract_bindings),
                _ => return Err(crate::v1::routes::invalid_path_error()),
            };
            #(#conversions)*
        }
    };

    // --- query extraction ---
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
                            .map_err(|e| {
                                ::http::Response::builder()
                                    .status(::http::StatusCode::BAD_REQUEST)
                                    .body(::bytes::Bytes::from(format!("invalid query: {}", e)))
                                    .unwrap()
                            })?;
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
                    .map_err(|e| {
                        ::http::Response::builder()
                            .status(::http::StatusCode::BAD_REQUEST)
                            .body(::bytes::Bytes::from(format!("invalid query: {}", e)))
                            .unwrap()
                    })?;
                #(let #named_idents = __qp.#named_idents;)*
            }
        };

        quote! {
            #named_extraction
            #(#stmts)*
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
                // Check if type is Option<T>
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
                            .ok_or_else(|| {
                                ::http::Response::builder()
                                    .status(::http::StatusCode::BAD_REQUEST)
                                    .body(::bytes::Bytes::from(
                                        format!("missing or invalid header: {}", #header_name)
                                    ))
                                    .unwrap()
                            })?;
                    }
                }
            })
            .collect();
        quote! { #(#stmts)* }
    };

    // --- json extraction ---
    let json_extraction = if let Some(f) = json_field {
        let ident = &f.ident;
        let ty = &f.ty;
        quote! {
            let #ident: #ty = ::serde_json::from_slice(&body)
                .map_err(|e| {
                    ::http::Response::builder()
                        .status(::http::StatusCode::BAD_REQUEST)
                        .body(::bytes::Bytes::from(format!("invalid json: {}", e)))
                        .unwrap()
                })?;
        }
    } else {
        quote! {}
    };

    // --- form extraction ---
    let form_extraction = if let Some(f) = form_field {
        let ident = &f.ident;
        let ty = &f.ty;
        quote! {
            let #ident: #ty = ::serde_urlencoded::from_str::<#ty>(
                &std::str::from_utf8(&body).map_err(|e| {
                    ::http::Response::builder()
                        .status(::http::StatusCode::BAD_REQUEST)
                        .body(::bytes::Bytes::from(format!("invalid form encoding: {}", e)))
                        .unwrap()
                })?
            ).map_err(|e| {
                ::http::Response::builder()
                    .status(::http::StatusCode::BAD_REQUEST)
                    .body(::bytes::Bytes::from(format!("invalid form data: {}", e)))
                    .unwrap()
            })?;
        }
    } else {
        quote! {}
    };

    // --- body extraction (raw bytes) ---
    let body_extraction = if let Some(f) = body_field {
        let ident = &f.ident;
        quote! {
            let #ident = body;
        }
    } else {
        quote! {}
    };

    // --- assemble Request struct ---
    let all_idents: Vec<_> = fields.iter().map(|f| &f.ident).collect();

    Ok(quote! {
        pub fn __extract(
            req: ::http::Request<::bytes::Bytes>,
        ) -> Result<Request, ::http::Response<::bytes::Bytes>> {
            let (parts, body) = req.into_parts();

            let path = parts.uri.path();

            // Extract query string
            let query_str = parts.uri.query().unwrap_or("");

            #path_extraction
            #query_extraction
            #header_extraction
            #json_extraction
            #form_extraction
            #body_extraction

            Ok(Request {
                #(#all_idents,)*
            })
        }
    })
}

// ---------------------------------------------------------------------------
// __encode
// ---------------------------------------------------------------------------

fn build_encode_fn(response_struct: &ItemStruct) -> syn::Result<TokenStream> {
    let has_json = extract_fields(response_struct)?
        .iter()
        .any(|f| matches!(f.kind, FieldKind::Json));

    if has_json {
        let json_field = extract_fields(response_struct)?
            .into_iter()
            .find(|f| matches!(f.kind, FieldKind::Json))
            .unwrap();
        let ident = &json_field.ident;

        Ok(quote! {
            pub fn __encode(resp: Response) -> ::http::Response<::bytes::Bytes> {
                let json = ::serde_json::to_string(&resp.#ident)
                    .unwrap_or_else(|e| format!("{{\"error\": \"serialization failed\"}}"));
                ::http::Response::builder()
                    .status(::http::StatusCode::OK)
                    .header(::http::header::CONTENT_TYPE, "application/json")
                    .body(::bytes::Bytes::from(json))
                    .unwrap()
            }
        })
    } else {
        Ok(quote! {
            pub fn __encode(resp: Response) -> ::http::Response<::bytes::Bytes> {
                ::http::Response::builder()
                    .status(::http::StatusCode::OK)
                    .body(::bytes::Bytes::new())
                    .unwrap()
            }
        })
    }
}

/// build the `route_module::meta()` function
fn build_meta_fn(args: &EndpointArgs, mod_attrs: &[&Attribute]) -> syn::Result<TokenStream> {
    let method_str = &args.method.value();
    // Convert to PascalCase (e.g., "GET" -> "Get", "POST" -> "Post")
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

    // Store original user-provided tags
    let user_tags: Vec<LitStr> = args.tags.clone();

    // Build full tags list including badge tags
    let mut tags_full: Vec<LitStr> = user_tags.clone();
    for s in &args.scopes {
        tags_full.push(LitStr::new(&format!("badge.scope.{}", s), s.span()));
    }
    for p in &args.permissions {
        tags_full.push(LitStr::new(&format!("badge.perm.{}", p), p.span()));
    }
    for p in &args.permissions_optional {
        tags_full.push(LitStr::new(&format!("badge.perm-opt.{}", p), p.span()));
    }

    let scopes: Vec<_> = args.scopes.iter().collect();
    let permissions: Vec<_> = args.permissions.iter().collect();
    let permissions_optional: Vec<_> = args.permissions_optional.iter().collect();
    let permissions_server: Vec<_> = args.permissions_server.iter().collect();
    let permissions_server_optional: Vec<_> = args.permissions_server_optional.iter().collect();

    // Parse doc comments into summary and description
    let (summary, description) = parse_doc_attrs(mod_attrs);

    Ok(quote! {
        pub fn metadata() -> crate::v1::routes::Endpoint {
            crate::v1::routes::Endpoint {
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
            }
        }
    })
}

fn parse_doc_attrs(attrs: &[&Attribute]) -> (LitStr, TokenStream) {
    let mut doc_lines: Vec<String> = Vec::new();

    for attr in attrs {
        if let syn::Meta::NameValue(nv) = &attr.meta {
            if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(lit),
                ..
            }) = &nv.value
            {
                let value = lit.value();
                let trimmed = value.strip_prefix(' ').unwrap_or(&value);
                doc_lines.push(trimmed.to_string());
            }
        }
    }

    let summary = LitStr::new(
        doc_lines.first().map(|s| s.as_str()).unwrap_or(""),
        Span::call_site(),
    );

    let description = if doc_lines.len() > 1 {
        let rest = &doc_lines[1..];
        let start = rest
            .iter()
            .position(|l| !l.is_empty())
            .unwrap_or(rest.len());
        let desc = rest[start..].join("\n");
        if desc.is_empty() {
            quote! { None }
        } else {
            quote! { Some(#desc) }
        }
    } else {
        quote! { None }
    };

    (summary, description)
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
            return Ok(FieldKind::Path(try_parse_header_or_path_arg(attr)));
        }
        if path.is_ident("query") {
            return Ok(FieldKind::Query(try_parse_header_or_path_arg(attr)));
        }
        if path.is_ident("header") {
            return Ok(FieldKind::Header(try_parse_header_or_path_arg(attr)));
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

fn try_parse_header_or_path_arg(attr: &Attribute) -> Option<String> {
    // Try parsing as rename = "..." first
    attr.parse_args::<syn::Meta>()
        .ok()
        .and_then(|meta| {
            if let syn::Meta::NameValue(nv) = meta {
                if nv.path.is_ident("rename") {
                    if let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(lit),
                        ..
                    }) = nv.value
                    {
                        return Some(lit.value());
                    }
                }
            }
            None
        })
        .or_else(|| {
            // Fall back to simple string literal
            attr.parse_args::<LitStr>().ok().map(|s| s.value())
        })
}

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

/// Build a match pattern for path extraction.
/// For "/user/{id}/posts/{post_id}" returns:
/// - pattern: `["", "user", id_raw, "posts", post_id_raw]`
/// - bindings: `id_raw, post_id_raw`
fn build_path_match_pattern(template: &str) -> syn::Result<(TokenStream, TokenStream)> {
    let mut pattern_items: Vec<TokenStream> = Vec::new();
    let mut bindings: Vec<TokenStream> = Vec::new();

    for segment in template.split('/') {
        if let Some(param_name) = segment.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
            // This is a path parameter
            let raw_name = format_ident!("{}_raw", param_name);
            pattern_items.push(quote! { #raw_name });
            bindings.push(quote! { #raw_name });
        } else {
            // This is a literal segment
            pattern_items.push(quote! { #segment });
        }
    }

    let pattern = quote! { [#(#pattern_items),*] };
    let bindings = quote! { #(#bindings),* };

    Ok((pattern, bindings))
}
