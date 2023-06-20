use proc_macro::{Span, TokenStream};
use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::{parse_macro_input, spanned::Spanned, Data::Enum, DeriveInput, Field, Fields};

use crate::bevy_ecs_path;

pub fn derive_states(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let error = || {
        syn::Error::new(
            Span::call_site().into(),
            "derive(States) only supports enums (additionaly, all of the fields need to implement States too)",
        )
        .into_compile_error()
        .into()
    };
    let Enum(enumeration) = ast.data else {
        return error();
    };
    let mut error: Option<syn::Error> = None;
    let non_unnamed = enumeration
        .variants
        .iter()
        .filter(|v| matches!(v.fields, Fields::Named(_)));
    for variant in non_unnamed {
        let err = syn::Error::new(
            variant.span(),
            format!(
                "Expected either unit (e.g. None) or unnamed field (e.g. Some(T)), found {}",
                match variant.fields {
                    Fields::Named(_) => "named structs (e.g. Foo { bar: bool }",
                    _ => unreachable!(),
                }
            ),
        );
        match &mut error {
            Some(error) => error.combine(err),
            None => error = Some(err),
        }
    }

    if let Some(error) = error {
        return error.into_compile_error().into();
    }

    {}
    let generics = ast.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let mut trait_path = bevy_ecs_path();
    trait_path.segments.push(format_ident!("schedule").into());
    trait_path.segments.push(format_ident!("States").into());
    let struct_name = &ast.ident;
    let fieldless_idents = enumeration
        .variants
        .iter()
        .filter(|v| v.fields.is_empty())
        .map(|v| &v.ident);
    let fieldful_variants = enumeration.variants.iter().filter(|v| !v.fields.is_empty());
    let fieldful_idents: Vec<&Ident> = fieldful_variants.clone().map(|v| &v.ident).collect();
    let fieldful_values: Vec<&Field> = fieldful_variants
        .flat_map(|v| match &v.fields {
            syn::Fields::Unnamed(field) => &field.unnamed,
            _ => unreachable!(),
        })
        .collect();

    let len = enumeration.variants.len();
    let (variants_impl, iter_type) = if !fieldful_idents.is_empty() {
        (
            quote! {

                [vec![#(Self::#fieldless_idents,)*], #(<#fieldful_values as #trait_path>::variants().map(|variant| {
                            Self::#fieldful_idents(variant)
                }).collect::<Vec<Self>>(),)*].into_iter()
                .flatten()
                .collect::<Vec<Self>>()
                .into_iter()
            },
            quote! {std::vec::IntoIter<Self>},
        )
    } else {
        (
            quote! {[#(Self::#fieldless_idents,)*].into_iter()  },
            quote! {std::array::IntoIter<Self, #len>},
        )
    };
    let token = quote! {
        impl #impl_generics #trait_path for #struct_name #ty_generics #where_clause {
            type Iter = #iter_type;

            fn variants() -> Self::Iter {
                #variants_impl
            }
        }
    };
    token.into()
}
