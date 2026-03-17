use proc_macro::TokenStream;

mod diff;
mod endpoint;
mod handler;
mod ids;
mod parse;

#[proc_macro_derive(Diff, attributes(diff))]
pub fn derive_diff(input: TokenStream) -> TokenStream {
    diff::expand_diff_derive(input)
}

#[proc_macro_attribute]
pub fn endpoint(args: TokenStream, item: TokenStream) -> TokenStream {
    endpoint::expand(args.into(), item.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn handler(args: TokenStream, item: TokenStream) -> TokenStream {
    handler::expand(args.into(), item.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro]
pub fn user_id(input: TokenStream) -> TokenStream {
    let lit = syn::parse_macro_input!(input as syn::LitStr);
    ids::expand_typed_id(lit, "UserId").into()
}

#[proc_macro]
pub fn room_id(input: TokenStream) -> TokenStream {
    let lit = syn::parse_macro_input!(input as syn::LitStr);
    ids::expand_typed_id(lit, "RoomId").into()
}

#[proc_macro]
pub fn channel_id(input: TokenStream) -> TokenStream {
    let lit = syn::parse_macro_input!(input as syn::LitStr);
    ids::expand_typed_id(lit, "ChannelId").into()
}
