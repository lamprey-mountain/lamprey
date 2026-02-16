use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, ItemMod};

#[proc_macro_attribute]
pub fn route(_args: TokenStream, item: TokenStream) -> TokenStream {
    // For now, just return the item as is or a simple wrapper to verify it compiles.
    // The full implementation will involve parsing args and generating code.
    let input = parse_macro_input!(item as ItemMod);

    // In a real implementation we would parse `_args` to get method, path, tags, etc.
    // and inspect `item` to find `Request` and `Response` structs and their fields.

    let expanded = quote! {
        #input
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn request(_args: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);

    // Boilerplate: In reality we might remove this attribute and implement traits.
    // For now, let's just pass it through.

    let expanded = quote! {
        #input
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn response(_args: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);

    let expanded = quote! {
        #input
    };

    TokenStream::from(expanded)
}

#[proc_macro]
pub fn user_id(input: TokenStream) -> TokenStream {
    let input_str = input.to_string();
    // Validate UUID here in a real impl
    // For now, just generating a UUID construction code
    let expanded = quote! {
        uuid::Uuid::parse_str(#input_str).expect("Invalid UUID constant")
    };
    TokenStream::from(expanded)
}

#[proc_macro]
pub fn room_id(input: TokenStream) -> TokenStream {
    let input_str = input.to_string();
    let expanded = quote! {
        uuid::Uuid::parse_str(#input_str).expect("Invalid UUID constant")
    };
    TokenStream::from(expanded)
}
