use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{FnArg, ItemFn, Pat, Path, parse2};

pub fn expand(args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    // TODO
    Ok(item)
}
