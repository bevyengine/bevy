use proc_macro2::Ident;
use quote::quote;
use syn::{Generics, Path};

pub(crate) fn impl_typed(
    type_name: &Ident,
    generics: &Generics,
    generator: proc_macro2::TokenStream,
    bevy_reflect_path: &Path,
) -> proc_macro2::TokenStream {
    let is_generic = !generics.params.is_empty();

    let static_generator = if is_generic {
        quote! {
            static CELL: #bevy_reflect_path::utility::GenericTypeInfoCell = #bevy_reflect_path::utility::GenericTypeInfoCell::new();
            CELL.get_or_insert::<Self, _>(|| {
                #generator
            })
        }
    } else {
        quote! {
            static CELL: #bevy_reflect_path::utility::NonGenericTypeInfoCell = #bevy_reflect_path::utility::NonGenericTypeInfoCell::new();
            CELL.get_or_set(|| {
                #generator
            })
        }
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics #bevy_reflect_path::Typed for #type_name #ty_generics #where_clause {
            fn type_info() -> &'static #bevy_reflect_path::TypeInfo {
                #static_generator
            }
        }
    }
}
