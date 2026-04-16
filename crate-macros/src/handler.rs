use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{parse2, spanned::Spanned, FnArg, ItemFn, Pat, Path};

pub fn expand(args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let endpoint_mod: Path = parse2(args)?;
    let func: ItemFn = parse2(item)?;

    let fn_name = &func.sig.ident;
    let fn_vis = &func.vis;
    let fn_attrs = &func.attrs;
    let fn_block = &func.block;
    let fn_output = &func.sig.output;
    let inner_name = format_ident!("__{fn_name}_inner");
    let all_inputs = &func.sig.inputs;

    let mut req_ident = quote! { __req };
    let mut req_found = false;
    let mut forward_args: Vec<TokenStream> = vec![];
    let mut outer_inputs: Vec<TokenStream> = vec![];

    for (idx, arg) in all_inputs.iter().enumerate() {
        if let FnArg::Typed(pt) = arg {
            if let syn::Type::Path(tp) = &*pt.ty {
                // check if type path starts with endpoint_mod and ends with ::Request
                let type_segs: Vec<_> = tp.path.segments.iter().collect();
                let mod_segs: Vec<_> = endpoint_mod.segments.iter().collect();
                let is_request = type_segs.len() == mod_segs.len() + 1
                    && type_segs
                        .last()
                        .map(|s| s.ident == "Request")
                        .unwrap_or(false)
                    && type_segs[..mod_segs.len()]
                        .iter()
                        .zip(mod_segs.iter())
                        .all(|(a, b)| a.ident == b.ident);
                if is_request {
                    let ident = if let Pat::Ident(pi) = &*pt.pat {
                        &pi.ident
                    } else {
                        &format_ident!("__req")
                    };
                    req_ident = quote! { #ident };
                    forward_args.push(quote! { #req_ident });
                    req_found = true;
                    continue;
                }
            }

            // Normal argument
            if let Pat::Ident(pi) = &*pt.pat {
                let ident = &pi.ident;
                forward_args.push(quote! { #ident });
                let ty = &pt.ty;
                outer_inputs.push(quote! { #ident: #ty });
                continue;
            }

            // Non-ident pattern (e.g. State(s)): generate a forwarding binding
            let ty = &pt.ty;
            let generated = format_ident!("__arg_{idx}");
            forward_args.push(quote! { #generated });
            outer_inputs.push(quote! { #generated: #ty });
            continue;
        }

        // unlikely but just in case
        if let FnArg::Receiver(_) = arg {
            forward_args.push(quote! { self });
        }
    }

    if !req_found {
        return Err(syn::Error::new(
            all_inputs.span(),
            "handler must take exactly one Request argument",
        ));
    }

    let fn_name_struct = {
        let pascal: String = fn_name
            .to_string()
            .split('_')
            .map(|seg| {
                let mut chars = seg.chars();
                match chars.next() {
                    None => String::new(),
                    Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect();
        format_ident!("{}Path", pascal)
    };

    Ok(quote! {
        async fn #inner_name(#all_inputs) #fn_output #fn_block

        #(#fn_attrs)*
        #fn_vis async fn #fn_name(
            #(#outer_inputs,)*
            __raw_req: ::axum::extract::Request,
        ) -> ::core::result::Result<
            ::axum::response::Response,
            ::axum::response::Response,
        > {
            use ::axum::response::IntoResponse as _;
            let (__parts, __body) = __raw_req.into_parts();
            let __bytes = if let Some(body) = __parts.extensions.get::<common::util::FederationBody>() {
                body.0.clone()
            } else {
                ::axum::body::to_bytes(__body, ::core::primitive::usize::MAX)
                    .await
                    .map_err(|_| ::axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response())?
            };
            let __bytes_req = ::http::Request::from_parts(__parts, __bytes);
            let #req_ident = #endpoint_mod::extract_request(__bytes_req).map_err(|e| {
                let (parts, body) = e.into_parts();
                ::axum::response::Response::from_parts(parts, ::axum::body::Body::from(body))
            })?;
            #inner_name(#(#forward_args),*)
                .await
                .map_err(|e| e.into_response())
                .map(|r| r.into_response())
        }

        #fn_vis struct #fn_name_struct;

        impl ::utoipa::Path for #fn_name_struct {
            fn methods() -> Vec<::utoipa::openapi::HttpMethod> {
                let meta = #endpoint_mod::metadata();
                vec![meta.method.into()]
            }

            fn path() -> String {
                #endpoint_mod::metadata().path.to_string()
            }

            fn operation() -> ::utoipa::openapi::path::Operation {
                let meta = #endpoint_mod::metadata();
                let mut op = ::utoipa::openapi::path::OperationBuilder::new()
                    .summary(Some(meta.summary))
                    .description(meta.description);
                for tag in meta.tags_full {
                    op = op.tag(*tag);
                }
                #endpoint_mod::update_operation(op).build()
            }
        }
    })
}
