use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parenthesized, parse::Parse, punctuated::Punctuated, token::Paren, AttrStyle, DataEnum,
    DeriveInput, Ident, MetaList, Token, Variant,
};

use crate::bevy_ecs_path;

pub fn derive_states(ast: DeriveInput) -> syn::Result<TokenStream> {
    let can_transit_to_impl = match &ast.data {
        syn::Data::Enum(data) => impl_can_transit_to_for_enum(&ast.ident, data)?,
        syn::Data::Struct(_) | syn::Data::Union(_) => quote! { true },
    };

    let generics = ast.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let mut trait_path = bevy_ecs_path();
    trait_path.segments.push(format_ident!("schedule").into());
    trait_path.segments.push(format_ident!("States").into());
    let struct_name = &ast.ident;

    Ok(quote! {
        impl #impl_generics #trait_path for #struct_name #ty_generics #where_clause {
            #[inline]
            fn can_transit_to(&self, target: &Self) -> bool {
                #can_transit_to_impl
            }
        }
    })
}

const TRANSITION_TO: &str = "transition_to";

fn impl_can_transit_to_for_enum(enum_name: &Ident, data: &DataEnum) -> syn::Result<TokenStream> {
    let match_branches = data
        .variants
        .iter()
        .map(|variant| {
            let variant_ident = &variant.ident;
            let transitions = extract_transition_attributes_for_variant(variant)?;

            Ok(match transitions {
                Some(TransitionTo { inverted, targets }) => {
                    let value = if inverted.is_some() {
                        quote!(false)
                    } else {
                        quote!(true)
                    };
                    let targets = targets.iter();
                    quote! {
                        #( ( #enum_name::#variant_ident, #enum_name::#targets ) => #value, )*
                        ( #enum_name::#variant_ident, _ ) => !#value,
                    }
                }

                // The default behaviour is to allow the transition.
                None => quote! {
                    ( #enum_name::#variant_ident, _ ) => true,
                },
            })
        })
        .collect::<syn::Result<TokenStream>>()?;

    Ok(quote! {
        match (self, target) {
            #match_branches
        }
    })
}

fn extract_transition_attributes_for_variant(
    variant: &Variant,
) -> syn::Result<Option<TransitionTo>> {
    let mut transitions = variant
        .attrs
        .iter()
        .filter(|attr| matches!(attr.style, AttrStyle::Outer))
        .filter_map(|attr| match &attr.meta {
            syn::Meta::List(MetaList { path, tokens, .. }) if path.is_ident(TRANSITION_TO) => {
                Some(syn::parse2(tokens.clone()))
            }
            _ => None,
        });

    let first = transitions.next().transpose();

    if transitions.next().is_some() {
        return Err(syn::Error::new_spanned(
            variant,
            format!("only one `{TRANSITION_TO}` attribute is allowed per variant"),
        ));
    }

    first
}

mod kw {
    syn::custom_keyword!(not);
}

struct TransitionTo {
    inverted: Option<(kw::not, Paren)>,
    targets: Punctuated<Ident, Token![,]>,
}

impl Parse for TransitionTo {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(if input.peek(kw::not) {
            let content;
            TransitionTo {
                inverted: Some((input.parse()?, parenthesized!(content in input))),
                targets: content.parse_terminated(Ident::parse, Token![,])?,
            }
        } else {
            TransitionTo {
                inverted: None,
                targets: input.parse_terminated(Ident::parse, Token![,])?,
            }
        })
    }
}
