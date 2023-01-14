use crate::fq_std::{FQAny, FQBox, FQClone, FQOption, FQResult};
use crate::impls::impl_typed;
use crate::utility::WhereClauseOptions;
use crate::ReflectMeta;
use proc_macro::TokenStream;
use quote::quote;

use super::impl_full_reflect;

/// Implements `GetTypeRegistration` and `Reflect` for the given type data.
pub(crate) fn impl_value(meta: &ReflectMeta) -> TokenStream {
    let bevy_reflect_path = meta.bevy_reflect_path();
    let type_name = meta.type_name();

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

    let where_clause_options = WhereClauseOptions::default();
    let typed_impl = impl_typed(
        type_name,
        meta.generics(),
        &where_clause_options,
        quote! {
            let info = #bevy_reflect_path::ValueInfo::new::<Self>() #with_docs;
            #bevy_reflect_path::TypeInfo::Value(info)
        },
        bevy_reflect_path,
    );
    
    let impl_full_reflect = impl_full_reflect(&meta);

    let (impl_generics, ty_generics, where_clause) = meta.generics().split_for_impl();
    
    let get_type_registration_impl = meta.get_type_registration(&where_clause_options);

    TokenStream::from(quote! {
        #impl_full_reflect
        
        #get_type_registration_impl

        #typed_impl

        impl #impl_generics #bevy_reflect_path::PartialReflect for #type_name #ty_generics #where_clause  {
            #[inline]
            fn type_name(&self) -> &str {
                ::core::any::type_name::<Self>()
            }

            #[inline]
            fn get_type_info(&self) -> &'static #bevy_reflect_path::TypeInfo {
                <Self as #bevy_reflect_path::Typed>::type_info()
            }

            #[inline]
            fn into_any(self: #FQBox<Self>) -> #FQBox<dyn #FQAny> {
                self
            }

            #[inline]
            fn as_any(&self) -> &dyn #FQAny {
                self
            }

            #[inline]
            fn as_any_mut(&mut self) -> &mut dyn #FQAny {
                self
            }

            #[inline]
            fn into_reflect(self: #FQBox<Self>) -> #FQBox<dyn #bevy_reflect_path::PartialReflect> {
                self
            }

            #[inline]
            fn as_reflect(&self) -> &dyn #bevy_reflect_path::PartialReflect {
                self
            }

            #[inline]
            fn as_reflect_mut(&mut self) -> &mut dyn #bevy_reflect_path::PartialReflect {
                self
            }

            #[inline]
            fn clone_value(&self) -> #FQBox<dyn #bevy_reflect_path::PartialReflect> {
                #FQBox::new(#FQClone::clone(self))
            }

            #[inline]
            fn apply(&mut self, value: &dyn #bevy_reflect_path::PartialReflect) {
                let value = #bevy_reflect_path::PartialReflect::as_any(value);
                if let #FQOption::Some(value) = <dyn #FQAny>::downcast_ref::<Self>(value) {
                    *self = #FQClone::clone(value);
                } else {
                    panic!("Value is not {}.", ::core::any::type_name::<Self>());
                }
            }

            #[inline]
            fn set(&mut self, value: #FQBox<dyn #bevy_reflect_path::PartialReflect>) -> #FQResult<(), #FQBox<dyn #bevy_reflect_path::PartialReflect>> {
                *self = <dyn #bevy_reflect_path::PartialReflect>::take(value)?;
                #FQResult::Ok(())
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
    })
}
