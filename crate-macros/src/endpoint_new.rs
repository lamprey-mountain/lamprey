// TODO: do this
//! work in progress rewrite for the endpoint macro

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse2, ItemMod, ItemStruct,
};

use crate::endpoint_new::parse::{EndpointArgs, EndpointModule};

pub fn expand(args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let args: EndpointArgs = parse2(args)?;
    let module: EndpointModule = parse2(item)?;

    // TODO: clean request/response structs
    // TODO: impl extract/encode functions

    // let request_clean = build_clean_struct(request_struct.clone())?;
    // let response_clean = build_clean_struct(response_struct.clone())?;
    // let extract_request_fn = build_extract_request_fn(&req_fields, &args.path)?;
    // let extract_impl = build_extract_impl(&req_fields, &args.path)?;
    // let encode_response_fn = build_encode_response_fn(&args, response_struct)?;
    // let encode_request_fn = build_encode_request_fn(&req_fields, &args.path, &args)?;
    // let extract_response_fn = build_extract_response_fn(&args, response_struct)?;
    // let meta_fn = build_meta_fn(&args, &mod_attrs)?;
    // let openapi_ext_fn = build_openapi_ext_fn(&args, &req_fields, &resp_fields)?;

    // // Preserve all original items except Request/Response structs
    // let preserved_items: Vec<_> = items
    //     .iter()
    //     .filter(|item| {
    //         if let Item::Struct(s) = item {
    //             s.ident != "Request" && s.ident != "Response"
    //         } else {
    //             true
    //         }
    //     })
    //     .collect();

    let expanded = quote! {
        mod mod_name {
            pub struct Endpoint;
            pub struct Request;
            pub struct Response;

            impl ::crate::util::routes::Endpoint for Endpoint {
                type Request = Request;
                type Response = Response;

                fn metadata() -> Metadata {
                    todo!()
                }
            }

            impl ::crate::util::routes::Request for Request {
                // TODO
            }

            impl ::crate::util::routes::Response for Response {
                // TODO
            }
        }
    };

    // let expanded = quote! {
    //     #(#mod_attrs)*
    //     #mod_vis mod #mod_name {
    //         use super::*;

    //         #[derive(Debug)]
    //         #request_clean

    //         #[derive(Debug)]
    //         #response_clean

    //         #extract_impl
    //         #extract_request_fn
    //         #encode_response_fn
    //         #encode_request_fn
    //         #extract_response_fn
    //         #meta_fn
    //         #openapi_ext_fn

    //         #(#preserved_items)*
    //     }
    // };

    let expanded2 = quote! {};

    Ok(expanded)
}

// fn extract_fields(s: &ItemStruct) -> syn::Result<Vec<EndpointField>> {}

// fn build_path_extraction(path_fields: &[&EndpointField], path: &LitStr) -> syn::Result<TokenStream> { todo!() }
// fn build_query_extraction(query_fields: &[&EndpointField]) -> TokenStream { todo!() }
// fn build_header_extraction(header_fields: &[&EndpointField]) -> TokenStream { todo!() }

mod parse {
    use syn::{
        parse::{Parse, ParseStream},
        ItemMod, ItemStruct,
    };

    pub use crate::parse::{EndpointArgs, EndpointField, FieldKind, ResponseSpec};

    pub struct EndpointModule {
        pub module: ItemMod,

        pub request: ItemStruct,
        pub response: ItemStruct,
    }

    impl Parse for EndpointModule {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let module: ItemMod = input.parse()?;
            // TODO: find request struct
            // TODO: find response struct

            // TODO: validate
            // - request may have at most one #[json], #[form], or #[body] field

            todo!()
        }
    }

    impl EndpointModule {
        /// get the `Request` struct without any attributes
        pub fn request_clean(&self) {
            todo!()
        }
    }
}
