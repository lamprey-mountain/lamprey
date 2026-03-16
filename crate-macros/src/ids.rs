use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, LitStr};

pub fn expand_typed_id(input: TokenStream, phantom_ty: &str) -> TokenStream {
    let lit = parse_macro_input!(input as LitStr);
    let s = lit.value();

    let uuid = match uuid::Uuid::parse_str(&s) {
        Ok(u) => u,
        Err(e) => {
            return syn::Error::new(lit.span(), format!("invalid UUID: {e}"))
                .to_compile_error()
                .into()
        }
    };

    let bytes = uuid.as_bytes();
    let b = bytes.iter().copied().collect::<Vec<u8>>();
    let phantom_ty =
        syn::parse_str::<syn::Type>(phantom_ty).expect("phantom_ty must be a valid Rust type");

    quote! {
        Id {
            inner: uuid::Uuid::from_bytes([#(#b),*]),
            phantom: std::marker::PhantomData::<#phantom_ty>,
        }
    }
    .into()
}
