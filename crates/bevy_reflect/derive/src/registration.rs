//! Contains code related specifically to Bevy's type registration.

use crate::{
    derive_data::ReflectMeta, serialization::SerializationDataDef,
    where_clause_options::WhereClauseOptions,
};
use quote::quote;
use syn::Type;

/// Creates the `GetTypeRegistration` impl for the given type data.
pub(crate) fn impl_get_type_registration<'a>(
    meta: &ReflectMeta,
    where_clause_options: &WhereClauseOptions,
    serialization_data: Option<&SerializationDataDef>,
    type_dependencies: Option<impl Iterator<Item = &'a Type>>,
) -> proc_macro2::TokenStream {
    let type_path = meta.type_path();
    let bevy_reflect_path = meta.bevy_reflect_path();

    let type_deps_fn = type_dependencies.map(|deps| {
        quote! {
            #[inline(never)]
            fn register_type_dependencies(registry: &mut #bevy_reflect_path::TypeRegistry) {
                #(<#deps as #bevy_reflect_path::__macro_exports::RegisterForReflection>::__register(registry);)*
            }
        }
    });

    let (impl_generics, ty_generics, where_clause) = type_path.generics().split_for_impl();
    let where_reflect_clause = where_clause_options.extend_where_clause(where_clause);

    let from_reflect_data = if meta.from_reflect().should_auto_derive() {
        Some(quote! {
            registration.insert(
                <#bevy_reflect_path::ReflectFromReflect as #bevy_reflect_path::CreateTypeData<Self>>::create_type_data(())
            );
        })
    } else {
        None
    };

    let serialization_data = serialization_data.map(|data| {
        let serialization_data = data.as_serialization_data(bevy_reflect_path);
        quote! {
            registration.insert::<#bevy_reflect_path::serde::SerializationData>(#serialization_data);
        }
    });

    let type_data = meta.attrs().type_data().iter().map(|data| {
        let reflect_ident = data.reflect_ident();
        let args = data.args();

        quote! {
            <#reflect_ident as #bevy_reflect_path::CreateTypeData<Self, _>>::create_type_data((#args))
        }
    });

    quote! {
        #[allow(unused_mut)]
        impl #impl_generics #bevy_reflect_path::GetTypeRegistration for #type_path #ty_generics #where_reflect_clause {
            fn get_type_registration() -> #bevy_reflect_path::TypeRegistration {
                let mut registration = #bevy_reflect_path::TypeRegistration::of::<Self>();
                registration.insert(
                    <#bevy_reflect_path::ReflectFromPtr as #bevy_reflect_path::CreateTypeData::<Self>>::create_type_data(())
                );
                #from_reflect_data
                #serialization_data
                #(registration.insert(#type_data);)*
                registration
            }

            #type_deps_fn
        }
    }
}
