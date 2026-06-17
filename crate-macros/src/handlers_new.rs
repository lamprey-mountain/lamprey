use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{ImplItem, ItemImpl, Path, parse2};

pub fn expand(_args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let mut input = parse2::<ItemImpl>(item)?;
    let self_ty = &input.self_ty;

    let mut registrations = Vec::new();

    for impl_item in &mut input.items {
        if let ImplItem::Fn(method) = impl_item {
            let endpoint_attr_pos = method
                .attrs
                .iter()
                .position(|a| a.path().is_ident("endpoint"));

            if let Some(idx) = endpoint_attr_pos {
                let attr = method.attrs.remove(idx);
                let metadata_path: Path = attr.parse_args()?;

                let method_name = &method.sig.ident;
                let handler_name = format_ident!("__{}_handler", method_name);

                let ep_type = quote! { #metadata_path::Endpoint };

                registrations.push(quote! {
                    {
                        async fn #handler_name(
                            ::axum::extract::State(globals): ::axum::extract::State<crate::util::Globals>,
                            req: crate::util::Req<#ep_type>,
                        ) -> Result<impl ::axum::response::IntoResponse> {
                            let this = <#self_ty>::new(globals);
                            this.#method_name(req).await.map(|r| {
                                use common::util::routes::Response as _;
                                r.encode().map(::axum::body::Body::from)
                            })
                        }

                        use common::util::routes::Endpoint as _;
                        let meta = #ep_type::metadata();

                        use crate::util::MethodExt as _;
                        r.route(
                            meta.path,
                            ::axum::routing::on(meta.method.to_filter(), #handler_name)
                        );

                        r.path(
                            meta.path,
                            #ep_type::path_item(),
                        );
                    }
                });
            }
        }
    }

    let expanded = quote! {
        #input

        impl crate::util::routes::Handlers for #self_ty {
            fn register(r: &mut crate::util::Routes) {
                #(#registrations)*
            }
        }
    };

    Ok(expanded)
}
