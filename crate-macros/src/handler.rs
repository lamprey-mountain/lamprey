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

    let forward_args: Vec<_> = non_req_inputs
        .iter()
        .map(|arg| {
            if let FnArg::Typed(pt) = arg {
                if let Pat::Ident(pi) = &*pt.pat {
                    return quote! { #pi };
                }
            }
            quote! { _ }
        })
        .collect();

    Ok(quote! {
        async fn #inner_name(#all_inputs) #fn_output #fn_block

        #(#fn_attrs)*
        #fn_vis async fn #fn_name(
            #(#non_req_inputs,)*
            __raw_req: ::axum::extract::Request,
        ) -> Result<impl ::axum::response::IntoResponse, ::axum::response::Response> {
            use ::axum::response::IntoResponse as _;
            let #req_ident = #endpoint_mod::__extract(__raw_req).await?;
            #inner_name(#(#forward_args,)* #req_ident)
                .await
                .map_err(|e| e.into_response())
        }
    })
}

fn pat_to_forward_arg(pat: &Pat) -> TokenStream {
    match pat {
        Pat::Ident(pi) => quote! { #pi },
        Pat::TupleStruct(pts) => {
            if let Some(inner) = pts.elems.first() {
                pat_to_forward_arg(inner)
            } else {
                quote! { _ }
            }
        }
        _ => quote! { _ },
    }
}
