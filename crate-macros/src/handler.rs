use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{parse2, FnArg, ItemFn, Pat, Path};

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
    let mut forward_args: Vec<TokenStream> = vec![];
    let mut outer_inputs: Vec<TokenStream> = vec![];
    let mut wants_universal = false;

    for (idx, arg) in all_inputs.iter().enumerate() {
        if let FnArg::Typed(pt) = arg {
            if let syn::Type::Path(tp) = &*pt.ty {
                let type_segs: Vec<_> = tp.path.segments.iter().collect();
                let mod_segs: Vec<_> = endpoint_mod.segments.iter().collect();

                // Check if it's the raw Request object
                let is_plain_request = type_segs.len() == mod_segs.len() + 1
                    && type_segs
                        .last()
                        .map(|s| s.ident == "Request")
                        .unwrap_or(false)
                    && type_segs[..mod_segs.len()]
                        .iter()
                        .zip(mod_segs.iter())
                        .all(|(a, b)| a.ident == b.ident);

                // Check if it's UniversalExtractor<Request>
                let mut is_universal_request = false;
                if !is_plain_request
                    && type_segs.len() == 1
                    && type_segs[0].ident == "UniversalExtractor"
                {
                    if let syn::PathArguments::AngleBracketed(args) = &type_segs[0].arguments {
                        if let Some(syn::GenericArgument::Type(syn::Type::Path(inner_tp))) =
                            args.args.first()
                        {
                            let inner_segs: Vec<_> = inner_tp.path.segments.iter().collect();
                            if inner_segs.len() == mod_segs.len() + 1
                                && inner_segs
                                    .last()
                                    .map(|s| s.ident == "Request")
                                    .unwrap_or(false)
                                && inner_segs[..mod_segs.len()]
                                    .iter()
                                    .zip(mod_segs.iter())
                                    .all(|(a, b)| a.ident == b.ident)
                            {
                                is_universal_request = true;
                            }
                        }
                    }
                }

                // If it matches either, intercept it so Axum doesn't try to extract it
                if is_plain_request || is_universal_request {
                    let ident = if let Pat::Ident(pi) = &*pt.pat {
                        &pi.ident
                    } else {
                        &format_ident!("__req")
                    };
                    req_ident = quote! { #ident };

                    if is_universal_request {
                        wants_universal = true;
                        forward_args.push(quote! { __extractor });
                    } else {
                        forward_args.push(quote! { #req_ident });
                    }

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

        if let FnArg::Receiver(_) = arg {
            forward_args.push(quote! { self });
        }
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

    let request_type_path = quote! { #endpoint_mod::Request };

    // TODO: extract automatically?
    let state_type = quote! { ::std::sync::Arc<crate::ServerState> };

    let unpack_stmt = if wants_universal {
        quote! {}
    } else {
        quote! { let #req_ident = __extractor.into_inner(); }
    };

    Ok(quote! {
        async fn #inner_name(#all_inputs) #fn_output #fn_block

        #(#fn_attrs)*
        #fn_vis async fn #fn_name(
            #(#outer_inputs,)*
            ::axum::extract::State(__state): ::axum::extract::State<#state_type>,
            __raw_req: ::axum::extract::Request,
        ) -> ::core::result::Result<
            ::axum::response::Response,
            ::axum::response::Response,
        > {
            use ::axum::response::IntoResponse as _;
            use ::axum::extract::FromRequest;

            let __extractor = crate::routes::util::body::UniversalExtractor::<#request_type_path>::from_request(
                __raw_req,
                &__state,
            )
            .await
            .map_err(|e| e.into_response())?;

            #unpack_stmt

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
