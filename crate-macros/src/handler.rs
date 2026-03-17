use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{parse2, FnArg, Ident, ItemFn, Pat};

pub fn expand(args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let endpoint_mod: Ident = parse2(args)?;
    let func: ItemFn = parse2(item)?;

    let fn_name = &func.sig.ident;
    let fn_vis = &func.vis;
    let fn_attrs = &func.attrs;
    let fn_block = &func.block;
    let fn_output = &func.sig.output;
    let inner_name = format_ident!("__{fn_name}_inner");
    let all_inputs = &func.sig.inputs;

    let mut req_ident = quote! { __req };
    let mut non_req_inputs: Vec<&FnArg> = vec![];

    for arg in all_inputs {
        if let FnArg::Typed(pt) = arg {
            if let syn::Type::Path(tp) = &*pt.ty {
                let segs: Vec<_> = tp.path.segments.iter().collect();
                if segs.len() == 2 && segs[0].ident == endpoint_mod && segs[1].ident == "Request" {
                    if let Pat::Ident(pi) = &*pt.pat {
                        req_ident = quote! { #pi };
                    }
                    continue;
                }
            }
        }
        non_req_inputs.push(arg);
    }

    let mut forward_args: Vec<_> = vec![];
    let mut outer_inputs: Vec<TokenStream> = vec![];

    for arg in &non_req_inputs {
        if let FnArg::Typed(pt) = arg {
            if let Pat::Ident(pi) = &*pt.pat {
                forward_args.push(quote! { #pi });
                outer_inputs.push(quote! { #arg });
                continue;
            }

            // Non-ident pattern (e.g. State(s)): generate a forwarding binding
            let ty = &pt.ty;
            let idx = outer_inputs.len();
            let generated = format_ident!("__arg_{idx}");
            forward_args.push(quote! { #generated });
            outer_inputs.push(quote! { #generated: #ty });
            continue;
        }

        // unlikely but just in case
        if let FnArg::Receiver(r) = arg {
            forward_args.push(quote! { self });
            outer_inputs.push(quote! { #r });
        }
    }

    Ok(quote! {
        async fn #inner_name(#all_inputs) #fn_output #fn_block
        #(#fn_attrs)*
        #fn_vis async fn #fn_name(
            #(#outer_inputs,)*
            __raw_req: ::axum::extract::Request,
        ) -> ::core::result::Result<
            impl ::axum::response::IntoResponse,
            ::axum::response::Response,
        > {
            use ::axum::response::IntoResponse as _;
            let (__parts, __body) = __raw_req.into_parts();
            let __bytes = ::axum::body::to_bytes(__body, ::core::primitive::usize::MAX)
                .await
                .map_err(|_| {
                    ::axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
                })?;
            let __bytes_req = ::http::Request::from_parts(__parts, __bytes);
            let #req_ident = #endpoint_mod::__extract(__bytes_req).map_err(|e| {
                let (parts, body) = e.into_parts();
                ::axum::response::Response::from_parts(parts, ::axum::body::Body::from(body))
            })?;
            #inner_name(#(#forward_args,)* #req_ident)
                .await
                .map_err(|e| e.into_response())
        }
    })
}
