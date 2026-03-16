use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse2, Ident, ItemFn};

pub fn expand(args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let _endpoint_mod: Ident = parse2(args)?;
    let func: ItemFn = parse2(item)?;
    let _fn_name = &func.sig.ident;
    let fn_vis = &func.vis;
    let fn_attrs = &func.attrs;

    // The generated function will:
    // 1. Accept individual axum extractors (Path, Query, Json, State, auth)
    // 2. Construct the endpoint::Request from them
    // 3. Call the original body
    // 4. Convert endpoint::Response into axum response

    // For now emit the fn as-is; a full impl would rewrite the signature
    Ok(quote! {
        #(#fn_attrs)*
        #fn_vis #func
    })
}
