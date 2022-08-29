//! Contains code related specifically to Bevy's type registration.

use crate::ReflectMeta;
use quote::quote;

/// Creates the `GetTypeRegistration` impl for the given type data.
pub(crate) fn impl_get_type_registration(meta: &ReflectMeta) -> proc_macro2::TokenStream {
    let type_name = meta.type_name();
    let bevy_reflect_path = meta.bevy_reflect_path();

    let registration_data = meta.traits().idents();

    let aliases = meta.traits().aliases();
    let deprecated_aliases = meta.traits().deprecated_aliases();

    let (impl_generics, ty_generics, where_clause) = meta.generics().split_for_impl();
    quote! {
        #[allow(unused_mut)]
        impl #impl_generics #bevy_reflect_path::GetTypeRegistration for #type_name #ty_generics #where_clause {
            fn get_type_registration() -> #bevy_reflect_path::TypeRegistration {
                let mut registration = #bevy_reflect_path::TypeRegistration::of::<#type_name #ty_generics>();
                registration.insert::<#bevy_reflect_path::ReflectFromPtr>(#bevy_reflect_path::FromType::<#type_name #ty_generics>::from_type());
                #(registration.insert::<#registration_data>(#bevy_reflect_path::FromType::<#type_name #ty_generics>::from_type());)*
                registration
            }

            fn aliases() -> &'static [&'static str] {
                &[#(#aliases),*]
            }

            fn deprecated_aliases() -> &'static [&'static str] {
                &[#(#deprecated_aliases),*]
            }
        }
    }
}
