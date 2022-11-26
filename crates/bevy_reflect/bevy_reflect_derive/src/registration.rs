//! Contains code related specifically to Bevy's type registration.

use crate::serialization::SerializationDenylist;
use crate::utility::{extend_where_clause, WhereClauseOptions};
use proc_macro2::Ident;
use quote::{quote, ToTokens};
use syn::{Generics, Path};

/// Creates the `GetTypeRegistration` impl for the given type data.
#[allow(clippy::too_many_arguments)]
pub(crate) fn impl_get_type_registration(
    type_name: &Ident,
    bevy_reflect_path: &Path,
    registration_data: &[Ident],
    generics: &Generics,
    where_clause_options: &WhereClauseOptions,
    serialization_denylist: Option<&SerializationDenylist>,
) -> proc_macro2::TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let serialization_data = serialization_denylist.map(|denylist| match denylist {
        SerializationDenylist::Struct(denylist) => {
            let denylist = denylist.into_iter();
            quote! {
                let ignored_indices = ::core::iter::IntoIterator::into_iter([#(#denylist),*]);
                registration.insert(
                    #bevy_reflect_path::serde::SerializationData::Struct(
                        #bevy_reflect_path::serde::StructSerializationData::new(ignored_indices)
                    )
                );
            }
        }
        SerializationDenylist::Enum(denylist) => {
            let mut indices = proc_macro2::TokenStream::new();
            for (variant_name, variant_fields) in denylist {
                let variant_fields = variant_fields.into_iter();
                quote!((
                    #variant_name,
                    ::core::iter::Iterator::copied(
                        ::core::iter::IntoIterator::into_iter(
                            &[#(#variant_fields),*] as &[usize]
                        )
                    )
                ),)
                .to_tokens(&mut indices);
            }
            quote! {
                let ignored_indices = ::core::iter::IntoIterator::into_iter([#indices]);
                registration.insert(
                    #bevy_reflect_path::serde::SerializationData::Enum(
                        #bevy_reflect_path::serde::EnumSerializationData::new(ignored_indices)
                    )
                );
            }
        }
    });

    let where_reflect_clause = extend_where_clause(where_clause, where_clause_options);

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
