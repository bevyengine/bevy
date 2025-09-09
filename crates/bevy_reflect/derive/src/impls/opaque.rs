use crate::{
    impls::{common_partial_reflect_methods, impl_full_reflect, impl_type_path, impl_typed},
    where_clause_options::WhereClauseOptions,
    ReflectMeta,
};
use bevy_macro_utils::fq_std::{FQClone, FQOption, FQResult};
use quote::quote;

/// Implements `GetTypeRegistration` and `Reflect` for the given type data.
pub(crate) fn impl_opaque(meta: &ReflectMeta) -> proc_macro2::TokenStream {
    let bevy_reflect_path = meta.bevy_reflect_path();
    let type_path = meta.type_path();

    #[cfg(feature = "documentation")]
    let with_docs = {
        let doc = quote::ToTokens::to_token_stream(meta.doc());
        Some(quote!(.with_docs(#doc)))
    };
    #[cfg(not(feature = "documentation"))]
    let with_docs: Option<proc_macro2::TokenStream> = None;

    let where_clause_options = WhereClauseOptions::new(meta);
    let typed_impl = impl_typed(
        &where_clause_options,
        quote! {
            let info = #bevy_reflect_path::OpaqueInfo::new::<Self>() #with_docs;
            #bevy_reflect_path::TypeInfo::Opaque(info)
        },
    );

    let type_path_impl = impl_type_path(meta);
    let full_reflect_impl = impl_full_reflect(&where_clause_options);
    let common_methods = common_partial_reflect_methods(meta, || None, || None);
    let clone_fn = meta.attrs().get_clone_impl(bevy_reflect_path);

    let apply_impl = if let Some(remote_ty) = meta.remote_ty() {
        let ty = remote_ty.type_path();
        quote! {
            if let #FQOption::Some(value) = <dyn #bevy_reflect_path::PartialReflect>::try_downcast_ref::<#ty>(value) {
                *self = Self(#FQClone::clone(value));
                return #FQResult::Ok(());
            }
        }
    } else {
        quote! {
            if let #FQOption::Some(value) = <dyn #bevy_reflect_path::PartialReflect>::try_downcast_ref::<Self>(value) {
                *self = #FQClone::clone(value);
                return #FQResult::Ok(());
            }
        }
    };

    #[cfg(not(feature = "functions"))]
    let function_impls = None::<proc_macro2::TokenStream>;
    #[cfg(feature = "functions")]
    let function_impls = crate::impls::impl_function_traits(&where_clause_options);

    #[cfg(not(feature = "auto_register"))]
    let auto_register = None::<proc_macro2::TokenStream>;
    #[cfg(feature = "auto_register")]
    let auto_register = crate::impls::reflect_auto_registration(meta);

    let (impl_generics, ty_generics, where_clause) = type_path.generics().split_for_impl();
    let where_reflect_clause = where_clause_options.extend_where_clause(where_clause);
    let get_type_registration_impl = meta.get_type_registration(&where_clause_options);

    quote! {
        #get_type_registration_impl

        #type_path_impl

        #typed_impl

        #full_reflect_impl

        #function_impls

        #auto_register

        impl #impl_generics #bevy_reflect_path::PartialReflect for #type_path #ty_generics #where_reflect_clause  {
            #[inline]
            fn get_represented_type_info(&self) -> #FQOption<&'static #bevy_reflect_path::TypeInfo> {
                #FQOption::Some(<Self as #bevy_reflect_path::Typed>::type_info())
            }

            #[inline]
            fn to_dynamic(&self) -> #bevy_reflect_path::__macro_exports::alloc_utils::Box<dyn #bevy_reflect_path::PartialReflect> {
                #bevy_reflect_path::__macro_exports::alloc_utils::Box::new(#FQClone::clone(self))
            }

             #[inline]
            fn try_apply(
                &mut self,
                value: &dyn #bevy_reflect_path::PartialReflect
            ) -> #FQResult<(), #bevy_reflect_path::ApplyError> {
                #apply_impl

                #FQResult::Err(
                    #bevy_reflect_path::ApplyError::MismatchedTypes {
                        from_type: ::core::convert::Into::into(#bevy_reflect_path::DynamicTypePath::reflect_type_path(value)),
                        to_type: ::core::convert::Into::into(<Self as #bevy_reflect_path::TypePath>::type_path()),
                    }
                )
            }

            #[inline]
            fn reflect_kind(&self) -> #bevy_reflect_path::ReflectKind {
                #bevy_reflect_path::ReflectKind::Opaque
            }

            #[inline]
            fn reflect_ref(&self) -> #bevy_reflect_path::ReflectRef {
                #bevy_reflect_path::ReflectRef::Opaque(self)
            }

            #[inline]
            fn reflect_mut(&mut self) -> #bevy_reflect_path::ReflectMut {
                #bevy_reflect_path::ReflectMut::Opaque(self)
            }

            #[inline]
            fn reflect_owned(self: #bevy_reflect_path::__macro_exports::alloc_utils::Box<Self>) -> #bevy_reflect_path::ReflectOwned {
                #bevy_reflect_path::ReflectOwned::Opaque(self)
            }

            #common_methods

            #clone_fn
        }
    }
}
