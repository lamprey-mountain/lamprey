use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub fn common_crate() -> TokenStream {
    match crate_name("lamprey-common").unwrap() {
        FoundCrate::Itself => quote! { crate },
        FoundCrate::Name(name) => {
            let ident = format_ident!("{}", name);
            quote! { ::#ident }
        }
    }
}
