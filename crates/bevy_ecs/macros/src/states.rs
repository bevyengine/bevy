use proc_macro::{Span, TokenStream};
use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data::Enum, DeriveInput};

use crate::bevy_ecs_path;

pub fn derive_states(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let error = || {
        syn::Error::new(
            Span::call_site().into(),
            "derive(States) only supports enums whose fields also implement States.",
        )
        .into_compile_error()
        .into()
    };
    let Enum(enumeration) = ast.data else {
        return error();
    };

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
    let fieldful_idents: Vec<&Ident> = enumeration
        .variants
        .iter()
        .filter(|v| !v.fields.is_empty())
        .map(|v| &v.ident)
        .collect();
    let len = enumeration.variants.len();
    let (variants_impl, iter_type) = if fieldful_idents.len() > 0 {
        (
            quote! {
                let mut fields = vec![#(Self::#fieldless_idents,)*];
                let fieldful = [#(<Self::#fieldful_idents as States>::variants().map(|variant| {
                            Self::#fieldful_idents(variant)
                }),)*];
                for field in fieldful {
                    fields.extend(field)
                }

                fields.into_iter()
            },
            quote! {std::vec::IntoIter<Self>},
        )
    } else {
        (
            quote! {[#(Self::#fieldless_idents,)*].into_iter()  },
            quote! {std::array::IntoIter<Self, #len>},
        )
    };
    quote! {
        impl #impl_generics #trait_path for #struct_name #ty_generics #where_clause {
            type Iter = #iter_type;

            fn variants() -> Self::Iter {
                #variants_impl
            }
        }
    }
    .into()
}
