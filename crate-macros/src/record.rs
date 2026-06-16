use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

// TODO: use #[record]
pub fn expand(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let expanded = quote! {
        #[derive(Debug, Clone)]
        #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
        #[cfg_attr(feature = "utoipa", derive(ToSchema))]
        #[cfg_attr(feature = "validator", derive(Validate))]
        #input
    };

    TokenStream::from(expanded)
}
