use crate::impls::{impl_type_path, impl_typed};
use crate::utility::{extend_where_clause, WhereClauseOptions};
use crate::ReflectMeta;
use bevy_macro_utils::fq_std::{FQBox, FQClone, FQOption, FQResult};
use quote::quote;

use super::impl_full_reflect;

/// Implements `GetTypeRegistration` and `Reflect` for the given type data.
pub(crate) fn impl_value(meta: &ReflectMeta) -> proc_macro2::TokenStream {
    let bevy_reflect_path = meta.bevy_reflect_path();
    let type_path = meta.type_path();

    let hash_fn = meta.traits().get_hash_impl(bevy_reflect_path);
    let partial_eq_fn = meta.traits().get_partial_eq_impl(bevy_reflect_path);
    let debug_fn = meta.traits().get_debug_impl();

    #[cfg(feature = "documentation")]
    let with_docs = {
        let doc = quote::ToTokens::to_token_stream(meta.doc());
        Some(quote!(.with_docs(#doc)))
    };
    #[cfg(not(feature = "documentation"))]
    let with_docs: Option<proc_macro2::TokenStream> = None;

    let where_clause_options = WhereClauseOptions::new_value(meta);
    let typed_impl = impl_typed(
        meta,
        &where_clause_options,
        quote! {
            let info = #bevy_reflect_path::ValueInfo::new::<Self>() #with_docs;
            #bevy_reflect_path::TypeInfo::Value(info)
        },
    );

    let full_reflect_impl = impl_full_reflect(meta, &where_clause_options);
    let type_path_impl = impl_type_path(meta, &where_clause_options);

    let (impl_generics, ty_generics, where_clause) = type_path.generics().split_for_impl();
    let where_reflect_clause = extend_where_clause(where_clause, &where_clause_options);
    let get_type_registration_impl = meta.get_type_registration(&where_clause_options);

    quote! {
        #get_type_registration_impl

        #type_path_impl

        #typed_impl

        #full_reflect_impl

        impl #impl_generics #bevy_reflect_path::PartialReflect for #type_path #ty_generics #where_reflect_clause  {
            #[inline]
            fn type_name(&self) -> &str {
                ::core::any::type_name::<Self>()
            }

            #[inline]
            fn get_represented_type_info(&self) -> #FQOption<&'static #bevy_reflect_path::TypeInfo> {
                #FQOption::Some(<Self as #bevy_reflect_path::Typed>::type_info())
            }

            fn try_as_reflect(&self) -> #FQOption<&dyn #bevy_reflect_path::Reflect> {
                #FQOption::Some(self)
            }

            fn try_as_reflect_mut(&mut self) -> #FQOption<&mut dyn #bevy_reflect_path::Reflect> {
                #FQOption::Some(self)
            }

            fn try_into_reflect(self: Box<Self>) -> #FQResult<Box<dyn #bevy_reflect_path::Reflect>, Box<dyn #bevy_reflect_path::PartialReflect>> {
                #FQResult::Ok(self)
            }

            fn as_partial_reflect(&self) -> &dyn #bevy_reflect_path::PartialReflect {
                self
            }

            fn as_partial_reflect_mut(&mut self) -> &mut dyn #bevy_reflect_path::PartialReflect {
                self
            }

            fn into_partial_reflect(self: #FQBox<Self>) -> #FQBox<dyn #bevy_reflect_path::PartialReflect> {
                self
            }

            #[inline]
            fn clone_value(&self) -> #FQBox<dyn #bevy_reflect_path::PartialReflect> {
                #FQBox::new(#FQClone::clone(self))
            }

            #[inline]
            fn apply(&mut self, value: &dyn #bevy_reflect_path::PartialReflect) {
                if let #FQOption::Some(value) = <dyn #bevy_reflect_path::PartialReflect>::try_downcast_ref::<Self>(value) {
                    *self = #FQClone::clone(value);
                } else {
                    panic!("Value is not {}.", ::core::any::type_name::<Self>());
                }
            }

            fn reflect_ref(&self) -> #bevy_reflect_path::ReflectRef {
                #bevy_reflect_path::ReflectRef::Value(self)
            }

            fn reflect_mut(&mut self) -> #bevy_reflect_path::ReflectMut {
                #bevy_reflect_path::ReflectMut::Value(self)
            }

            fn reflect_owned(self: #FQBox<Self>) -> #bevy_reflect_path::ReflectOwned {
                #bevy_reflect_path::ReflectOwned::Value(self)
            }

            #hash_fn

            #partial_eq_fn

            #debug_fn
        }
    }
}
