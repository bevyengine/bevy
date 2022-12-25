//! Contains code related specifically to Bevy's type registration.

use bit_set::BitSet;
use proc_macro2::Ident;
use quote::quote;
use syn::{Generics, Path};

/// Creates the `GetTypeRegistration` impl for the given type data.
pub(crate) fn impl_get_type_registration(
    type_name: &Ident,
    bevy_reflect_path: &Path,
    registration_data: &[Ident],
    generics: &Generics,
    serialization_denylist: Option<&BitSet<u32>>,
) -> proc_macro2::TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let serialization_data = serialization_denylist.map(|denylist| {
        let denylist = denylist.into_iter();
        quote! {
            let ignored_indices = ::core::iter::IntoIterator::into_iter([#(#denylist),*]);
            registration.insert::<#bevy_reflect_path::serde::SerializationData>(#bevy_reflect_path::serde::SerializationData::new(ignored_indices));
        }
    });

    quote! {
        #[allow(unused_mut)]
        impl #impl_generics #bevy_reflect_path::GetTypeRegistration for #type_name #ty_generics #where_clause {
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
