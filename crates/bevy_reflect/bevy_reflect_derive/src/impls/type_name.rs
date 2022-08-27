use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Generics, Path};

pub(crate) fn impl_type_name(
    type_name: &Ident,
    generics: &Generics,
    reflected_type_name: Option<&str>,
    bevy_reflect_path: &Path,
) -> proc_macro2::TokenStream {
    let is_generic = !generics.params.is_empty();

    let base_name = reflected_type_name
        .map(|x| quote!(#x))
        .unwrap_or_else(|| quote!(concat!(module_path!(), "::", stringify!(#type_name))));

    let get_type_name = if is_generic {
        let values = {
            let mut getters = generics.params.iter().map(|p| match p {
                syn::GenericParam::Type(p) => {
                    let ty = &p.ident;
                    quote!(<#ty as #bevy_reflect_path::TypeName>::name())
                }
                syn::GenericParam::Lifetime(p) => {
                    let name = &p.lifetime.ident;
                    quote!(concat!("'", stringify!(#name)))
                }
                syn::GenericParam::Const(p) => {
                    let name = &p.ident;
                    quote!(#name)
                }
            });

            // FIXME: Iterator::intersperse can be used here
            // currently unstable https://github.com/rust-lang/rust/issues/79524

            let mut values = Vec::with_capacity(generics.params.len());
            for _ in 0..generics.params.len() - 1 {
                // SAFETY: don't panic because we consume all but one item.
                let x = getters.next().unwrap();
                values.push(quote! {#x,});
            }
            // SAFETY: don't panic because the previous for loop didn't consume the last element
            // and there is at least one generic parameter.
            values.push(getters.next().unwrap());
            values.into_iter().collect::<TokenStream>()
        };

        let brackets = {
            // FIXME: Iterator::intersperse can be used here
            // currently unstable https://github.com/rust-lang/rust/issues/79524

            let mut brackets = vec!["{}, "; generics.params.len()];
            *brackets.last_mut().unwrap() = "{}";
            brackets.into_iter().collect::<String>()
        };

        quote! {
            static CELL: #bevy_reflect_path::utility::GenericTypeNameCell = #bevy_reflect_path::utility::GenericTypeNameCell::new();
            CELL.get_or_insert::<Self, _>(|| {
                format!(concat!("{}<", #brackets, ">"), BASE_NAME, #values)
            })
        }
    } else {
        quote! {
            BASE_NAME
        }
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics #bevy_reflect_path::TypeName for #type_name #ty_generics #where_clause {
            fn name() -> &'static str {
                const BASE_NAME: &'static str = #base_name;
                #get_type_name
            }
        }
    }
}
