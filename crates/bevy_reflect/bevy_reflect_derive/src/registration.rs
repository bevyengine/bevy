//! Contains code related specifically to Bevy's type registration.

use crate::utility::{extend_where_clause, WhereClauseOptions};
use bit_set::BitSet;
use proc_macro2::Ident;
use quote::quote;
use syn::{Generics, Path};

use crate::derive_data::{PathToType, ReflectMeta};

/// Creates the `GetTypeRegistration` impl for the given type data.
#[allow(clippy::too_many_arguments)]
pub(crate) fn impl_get_type_registration(
    meta: &ReflectMeta,
    where_clause_options: &WhereClauseOptions,
    serialization_denylist: Option<&BitSet<u32>>,
) -> proc_macro2::TokenStream {
    let path_to_type = meta.path_to_type();
    let bevy_reflect_path = meta.bevy_reflect_path();
    let registration_data = meta.traits().idents();
    let (impl_generics, ty_generics, where_clause) = meta.split_generics_for_impl();
    let serialization_data = serialization_denylist.map(|denylist| {
        let denylist = denylist.into_iter();
        quote! {
            let ignored_indices = ::core::iter::IntoIterator::into_iter([#(#denylist),*]);
            registration.insert::<#bevy_reflect_path::serde::SerializationData>(#bevy_reflect_path::serde::SerializationData::new(ignored_indices));
        }
    });

    let where_reflect_clause = extend_where_clause(where_clause, where_clause_options);

    quote! {
        #[allow(unused_mut)]
        impl #impl_generics #bevy_reflect_path::GetTypeRegistration for #path_to_type #ty_generics #where_reflect_clause {
            fn get_type_registration() -> #bevy_reflect_path::TypeRegistration {
                let mut registration = #bevy_reflect_path::TypeRegistration::of::<Self>();
                registration.insert::<#bevy_reflect_path::ReflectFromPtr>(#bevy_reflect_path::FromType::<Self>::from_type());
                #serialization_data
                #(registration.insert::<#registration_data>(#bevy_reflect_path::FromType::<Self>::from_type());)*
                registration
            }
        }
    }
}
