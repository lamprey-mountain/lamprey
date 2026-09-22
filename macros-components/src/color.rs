use std::str::FromStr;

use crate::util::common_crate;
use lamprey::v1::types::misc::{
    Color,
    color::{ColorName, ColorNamed, ColorOklch, ColorSrgb, ColorVariant},
};
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{Ident, LitStr, parse2};

pub fn expand(item: TokenStream) -> syn::Result<TokenStream> {
    expand_str(parse2(item)?)
}

pub fn expand_str(s: LitStr) -> syn::Result<TokenStream> {
    let krate = common_crate();
    let modd = quote! { #krate::v1::types::misc::color };

    let color_str = s.value();

    let color = match Color::from_str(&color_str) {
        Ok(color) => color,
        Err(err) => {
            return Err(syn::Error::new(s.span(), format!("Invalid color: {err}")));
        }
    };

    match color {
        Color::Srgb(c) => {
            let r = c.r;
            let g = c.g;
            let b = c.b;
            let alpha = alpha_to_tokens(c.alpha);
            Ok(quote! {
                #modd::Color::Srgb(#modd::ColorSrgb {
                    r: #r,
                    g: #g,
                    b: #b,
                    alpha: #alpha,
                })
            })
        }
        Color::Oklch(c) => {
            let l = c.l;
            let ci = c.c;
            let h = c.h;
            let alpha = alpha_to_tokens(c.alpha);
            Ok(quote! {
                #modd::Color::Srgb(#modd::ColorOklch {
                    l: #l,
                    c: #ci,
                    h: #h,
                    alpha: #alpha,
                })
            })
        }
        Color::Named(c) => {
            let name_str = c.name.to_string();
            let name = syn::parse_str::<syn::Path>(&format!(
                "{}::v1::types::misc::color::ColorName::{}",
                krate.to_token_stream(),
                name_str
                    .split('-')
                    .map(|s| {
                        let mut c = s.chars();
                        match c.next() {
                            None => String::new(),
                            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                        }
                    })
                    .collect::<String>()
            ))
            .expect("Failed to parse ColorName path");
            let variant = c.variant.value();
            let alpha = alpha_to_tokens(c.alpha);
            Ok(quote! {
                #modd::Color::Named(#modd::ColorNamed {
                    name: #name,
                    variant: #modd::ColorVariant::new_unchecked(#variant),
                    alpha: #alpha,
                })
            })
        }
        Color::Mystery(_) => Err(syn::Error::new(
            s.span(),
            "Invalid color: mystery colors are not allowed!",
        )),
    }
}

fn alpha_to_tokens(alpha: Option<f32>) -> TokenStream {
    match alpha {
        Some(a) => quote! { Some(#a) },
        None => quote! { None },
    }
}
