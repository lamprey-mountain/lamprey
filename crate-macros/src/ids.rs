use proc_macro::TokenStream;
use quote::quote;
use syn::LitStr;

pub fn expand_typed_id(lit: LitStr, phantom_ty: &str) -> TokenStream {
    let s = lit.value();

    let uuid = match uuid::Uuid::parse_str(&s) {
        Ok(u) => u,
        Err(e) => {
            return syn::Error::new(lit.span(), format!("invalid UUID: {e}"))
                .to_compile_error()
                .into()
        }
    };

    let b = uuid.as_bytes();
    let marker_ty = phantom_ty.strip_suffix("Id").unwrap_or(phantom_ty);
    let marker_ty = format!("Marker{}", marker_ty);
    let marker_ty =
        syn::parse_str::<syn::Type>(&marker_ty).expect("marker_ty must be a valid Rust type");

    quote! {
        Id {
            inner: ::uuid::Uuid::from_bytes([#(#b),*]),
            phantom: ::std::marker::PhantomData::<#marker_ty>,
        }
    }
    .into()
}
