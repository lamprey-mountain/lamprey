use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{ItemFn, Path, parse2};

pub fn expand(args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let metadata_path = parse2::<Path>(args)?;
    let input = parse2::<ItemFn>(item)?;

    let fn_name = &input.sig.ident;
    let handler_name = format_ident!("__{}_handler", fn_name);
    let ep_type = quote! { #metadata_path::Endpoint };

    Ok(quote! {
        #input

        ::inventory::submit! {
            crate::util::routes::Handler {
                tag: if #ep_type::metadata().tags.contains(&"cdn") {
                    "cdn"
                } else {
                    "api"
                },
                register: |r| {
                    async fn #handler_name(
                        req: crate::util::Req<#ep_type>,
                    ) -> Result<impl ::axum::response::IntoResponse> {
                        #fn_name(req).await.map(|r| {
                            use common::util::routes::Response as _;
                            r.encode().map(::axum::body::Body::from)
                        })
                    }
                    use common::util::routes::Endpoint as _;
                    let meta = #ep_type::metadata();
                    use crate::util::MethodExt as _;
                    r.route(
                        meta.path,
                        ::axum::routing::on(meta.method.to_filter(), #handler_name),
                    );
                    r.path(
                        meta.path,
                        #ep_type::path_item(),
                    );
                },
            }
        }
    })
}
