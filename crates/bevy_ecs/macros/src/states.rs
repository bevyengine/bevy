use proc_macro::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data::Enum, DeriveInput};

use crate::bevy_ecs_path;

pub fn derive_states(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let error = || {
        syn::Error::new(
            Span::call_site().into(),
            "derive(States) only supports fieldless enums",
        )
        .into_compile_error()
        .into()
    };
    let Enum(enumeration) = ast.data else {
        return error();
    };
    if enumeration.variants.iter().any(|v| !v.fields.is_empty()) {
        return error();
    }

    let generics = ast.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let mut trait_path = bevy_ecs_path();
    trait_path.segments.push(format_ident!("schedule").into());
    trait_path.segments.push(format_ident!("States").into());
    let struct_name = &ast.ident;
    let idents = enumeration.variants.iter().map(|v| &v.ident);
    let len = idents.len();

    quote! {
        impl #impl_generics #trait_path for #struct_name #ty_generics #where_clause {
            type Iter = std::array::IntoIter<Self, #len>;

            fn variants() -> Self::Iter {
                [#(Self::#idents,)*].into_iter()
            }
        }
    }
    .into()
}
