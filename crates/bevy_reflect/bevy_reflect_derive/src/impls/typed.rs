use proc_macro2::Ident;
use quote::quote;
use syn::{Generics, Path, Type};

pub(crate) fn impl_typed(
    type_name: &Ident,
    generics: &Generics,
    field_types: &Vec<Type>,
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

    // Add Typed bound for each active field
    let mut where_typed_clause = if where_clause.is_some() {
        quote! {#where_clause}
    } else if !field_types.is_empty() {
        quote! {where}
    } else {
        quote! {}
    };
    where_typed_clause.extend(quote! {
        #(#field_types: #bevy_reflect_path::Typed,)*
    });

    quote! {
        impl #impl_generics #bevy_reflect_path::Typed for #type_name #ty_generics #where_typed_clause {
            fn type_info() -> &'static #bevy_reflect_path::TypeInfo {
                #static_generator
            }
        }
    }
}
