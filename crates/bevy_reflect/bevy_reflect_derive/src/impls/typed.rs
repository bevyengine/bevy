use crate::utility::extend_where_clause;
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Generics, Path, Type};

pub(crate) fn impl_typed(
    type_name: &Ident,
    generics: &Generics,
    active_types: &[Type],
    ignored_types: &[Type],
    active_trait_bounds: &TokenStream,
    ignored_trait_bounds: &TokenStream,
    generator: TokenStream,
    bevy_reflect_path: &Path,
) -> TokenStream {
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

    // Add Typed bound for each active field
    let where_reflect_clause = extend_where_clause(
        where_clause,
        active_types,
        active_trait_bounds,
        ignored_types,
        ignored_trait_bounds,
    );

    quote! {
        impl #impl_generics #bevy_reflect_path::Typed for #type_name #ty_generics #where_reflect_clause {
            fn type_info() -> &'static #bevy_reflect_path::TypeInfo {
                #static_generator
            }
        }
    }
}
