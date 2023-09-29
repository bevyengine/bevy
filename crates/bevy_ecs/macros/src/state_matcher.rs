use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse::Parse, parse_macro_input, Ident, Pat, Token, Visibility};

use crate::bevy_ecs_path;

struct StateMatcher {
    visibility: Visibility,
    name: Ident,
    state_type: Ident,
    pattern: Pat,
}

impl Parse for StateMatcher {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let visibility: Visibility = input.parse()?;
        let name: Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let state_type: Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let pattern: Pat = Pat::parse_multi_with_leading_vert(input)?;

        Ok(Self {
            visibility,
            name,
            state_type,
            pattern,
        })
    }
}

pub fn define_state_matcher(input: TokenStream) -> TokenStream {
    let StateMatcher {
        visibility,
        name,
        state_type,
        pattern,
    } = parse_macro_input!(input as StateMatcher);

    let mut trait_path = bevy_ecs_path();
    trait_path.segments.push(format_ident!("schedule").into());
    trait_path
        .segments
        .push(format_ident!("StateMatcher").into());

    quote! {

       #[derive(Debug, Eq, PartialEq, Hash, Clone)]
        #visibility struct #name;

        impl #trait_path<#state_type> for #name {
            fn match_state(&self, state: &#state_type) -> bool {
                match state {
                    #pattern => true,
                    _ => false
                }
            }
        }

    }
    .into()
}
