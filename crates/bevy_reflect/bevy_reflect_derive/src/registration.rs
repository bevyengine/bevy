//! Contains code related specifically to Bevy's type registration.

use crate::derive_data::ReflectMeta;
use crate::fq_std::FQOption;
use crate::utility::{extend_where_clause, WhereClauseOptions};
use bit_set::BitSet;
use quote::quote;

/// Creates the `GetTypeRegistration` impl for the given type data.
#[allow(clippy::too_many_arguments)]
pub(crate) fn impl_get_type_registration(
    meta: &ReflectMeta,
    where_clause_options: &WhereClauseOptions,
    serialization_denylist: Option<&BitSet<u32>>,
) -> proc_macro2::TokenStream {
    let type_path = meta.type_path();
    let bevy_reflect_path = meta.bevy_reflect_path();
    let registration_data = meta.traits().idents();
    let (impl_generics, ty_generics, where_clause) = type_path.generics().split_for_impl();
    let where_reflect_clause = extend_where_clause(where_clause, where_clause_options);

    let from_reflect_data = if meta.from_reflect().should_auto_derive() {
        Some(quote! {
            .register::<Self, #bevy_reflect_path::ReflectFromReflect>()
        })
    } else {
        None
    };

    let serialization_data = serialization_denylist.map(|denylist| {
        let denylist = denylist.into_iter();
        quote! {
            .insert(#bevy_reflect_path::serde::SerializationData::new({
                let ignored_indices = ::core::iter::IntoIterator::into_iter([#(#denylist),*]);
                ignored_indices
            }), #FQOption::None)
        }
    });

    quote! {
        #[allow(unused_mut)]
        impl #impl_generics #bevy_reflect_path::GetTypeRegistration for #type_path #ty_generics #where_reflect_clause {
            fn get_type_registration() -> #bevy_reflect_path::TypeRegistration {
                #bevy_reflect_path::TypeRegistration::of::<Self>()
                    .register::<Self, #bevy_reflect_path::ReflectFromPtr>()
                    #from_reflect_data
                    #serialization_data
                    #(.register::<Self, #registration_data>())*
            }
        }
    }
}
