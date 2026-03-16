use proc_macro::TokenStream;

use crate::ids::expand_typed_id;

mod endpoint;
mod handler;
mod ids;
mod parse;

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
    expand_typed_id(input, "UserId")
}

#[proc_macro]
pub fn room_id(input: TokenStream) -> TokenStream {
    expand_typed_id(input, "RoomId")
}
