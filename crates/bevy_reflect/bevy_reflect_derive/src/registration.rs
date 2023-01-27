//! Contains code related specifically to Bevy's type registration.

use crate::utility::extend_where_clause;
use bit_set::BitSet;
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Generics, Path, Type};

/// Creates the `GetTypeRegistration` impl for the given type data.
#[allow(clippy::too_many_arguments)]
pub(crate) fn impl_get_type_registration(
    type_name: &Ident,
    bevy_reflect_path: &Path,
    registration_data: &[Ident],
    generics: &Generics,
    active_types: &[Type],
    ignored_types: &[Type],
    active_trait_bounds: &TokenStream,
    ignored_trait_bounds: &TokenStream,
    serialization_denylist: Option<&BitSet<u32>>,
) -> TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let serialization_data = serialization_denylist.map(|denylist| {
        let denylist = denylist.into_iter();
        quote! {
            let ignored_indices = ::core::iter::IntoIterator::into_iter([#(#denylist),*]);
            registration.insert::<#bevy_reflect_path::serde::SerializationData>(#bevy_reflect_path::serde::SerializationData::new(ignored_indices));
        }
    });

    let where_reflect_clause = extend_where_clause(
        where_clause,
        active_types,
        active_trait_bounds,
        ignored_types,
        ignored_trait_bounds,
    );

    quote! {
        #[allow(unused_mut)]
        impl #impl_generics #bevy_reflect_path::GetTypeRegistration for #type_name #ty_generics #where_reflect_clause {
            fn get_type_registration() -> #bevy_reflect_path::TypeRegistration {
                let mut registration = #bevy_reflect_path::TypeRegistration::of::<#type_name #ty_generics>();
                registration.insert::<#bevy_reflect_path::ReflectFromPtr>(#bevy_reflect_path::FromType::<#type_name #ty_generics>::from_type());
                #serialization_data
                #(registration.insert::<#registration_data>(#bevy_reflect_path::FromType::<#type_name #ty_generics>::from_type());)*
                registration
            }
        }
    }
}
