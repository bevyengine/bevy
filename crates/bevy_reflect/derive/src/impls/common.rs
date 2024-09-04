use bevy_macro_utils::fq_std::{FQAny, FQBox, FQOption, FQResult};

use quote::{quote, ToTokens};

use crate::{derive_data::ReflectMeta, utility::WhereClauseOptions};

pub fn impl_full_reflect(
    meta: &ReflectMeta,
    where_clause_options: &WhereClauseOptions,
) -> proc_macro2::TokenStream {
    let bevy_reflect_path = meta.bevy_reflect_path();
    let type_path = meta.type_path();

    let (impl_generics, ty_generics, where_clause) = type_path.generics().split_for_impl();
    let where_reflect_clause = where_clause_options.extend_where_clause(where_clause);

    let any_impls = if meta.is_remote_wrapper() {
        quote! {
            #[inline]
            fn into_any(self: #FQBox<Self>) -> #FQBox<dyn #FQAny> {
                #FQBox::new(self.0)
            }

            #[inline]
            fn as_any(&self) -> &dyn #FQAny {
                &self.0
            }

            #[inline]
            fn as_any_mut(&mut self) -> &mut dyn #FQAny {
                &mut self.0
            }
        }
    } else {
        quote! {
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
        }
    };

    quote! {
        impl #impl_generics #bevy_reflect_path::Reflect for #type_path #ty_generics #where_reflect_clause {
            #any_impls

            #[inline]
            fn into_reflect(self: #FQBox<Self>) -> #FQBox<dyn #bevy_reflect_path::Reflect> {
                self
            }

            #[inline]
            fn as_reflect(&self) -> &dyn #bevy_reflect_path::Reflect {
                self
            }

            #[inline]
            fn as_reflect_mut(&mut self) -> &mut dyn #bevy_reflect_path::Reflect {
                self
            }

            #[inline]
            fn set(
                &mut self,
                value: #FQBox<dyn #bevy_reflect_path::Reflect>
            ) -> #FQResult<(), #FQBox<dyn #bevy_reflect_path::Reflect>> {
                *self = <dyn #bevy_reflect_path::Reflect>::take(value)?;
                #FQResult::Ok(())
            }
        }
    }
}

pub fn common_partial_reflect_methods(
    meta: &ReflectMeta,
    default_partial_eq_delegate: impl FnOnce() -> Option<proc_macro2::TokenStream>,
    default_hash_delegate: impl FnOnce() -> Option<proc_macro2::TokenStream>,
) -> proc_macro2::TokenStream {
    let bevy_reflect_path = meta.bevy_reflect_path();

    let debug_fn = meta.attrs().get_debug_impl();
    let partial_eq_fn = meta
        .attrs()
        .get_partial_eq_impl(bevy_reflect_path)
        .or_else(move || {
            let default_delegate = default_partial_eq_delegate();
            default_delegate.map(|func| {
                quote! {
                    fn reflect_partial_eq(&self, value: &dyn #bevy_reflect_path::PartialReflect) -> #FQOption<bool> {
                        (#func)(self, value)
                    }
                }
            })
        });
    let hash_fn = meta
        .attrs()
        .get_hash_impl(bevy_reflect_path)
        .or_else(move || {
            let default_delegate = default_hash_delegate();
            default_delegate.map(|func| {
                quote! {
                    fn reflect_hash(&self) -> #FQOption<u64> {
                        (#func)(self)
                    }
                }
            })
        });

    quote! {
        #[inline]
        fn try_into_reflect(
            self: #FQBox<Self>
        ) -> #FQResult<#FQBox<dyn #bevy_reflect_path::Reflect>, #FQBox<dyn #bevy_reflect_path::PartialReflect>> {
            #FQResult::Ok(self)
        }

        #[inline]
        fn try_as_reflect(&self) -> #FQOption<&dyn #bevy_reflect_path::Reflect> {
            #FQOption::Some(self)
        }

        #[inline]
        fn try_as_reflect_mut(&mut self) -> #FQOption<&mut dyn #bevy_reflect_path::Reflect> {
            #FQOption::Some(self)
        }

        #[inline]
        fn into_partial_reflect(self: #FQBox<Self>) -> #FQBox<dyn #bevy_reflect_path::PartialReflect> {
            self
        }

        #[inline]
        fn as_partial_reflect(&self) -> &dyn #bevy_reflect_path::PartialReflect {
            self
        }

        #[inline]
        fn as_partial_reflect_mut(&mut self) -> &mut dyn #bevy_reflect_path::PartialReflect {
            self
        }

        #hash_fn

        #partial_eq_fn

        #debug_fn
    }
}

pub fn reflect_auto_registration(meta: &ReflectMeta) -> Option<proc_macro2::TokenStream> {
    if meta.attrs().no_auto_register() {
        return None;
    }

    let bevy_reflect_path = meta.bevy_reflect_path();
    let type_path = meta.type_path();
    let (_, ty_generics, _) = meta.type_path().generics().split_for_impl();

    if !ty_generics.into_token_stream().is_empty() {
        return None;
    };

    Some(quote! {
        #[cfg(target_family = "wasm")]
        #bevy_reflect_path::wasm_init::wasm_init!{
            use #bevy_reflect_path::{GetTypeRegistration, TypeRegistration};
            #bevy_reflect_path::DERIVED_REFLECT_TYPES
                .write()
                .unwrap()
                .push((TypeRegistration::of::<#type_path>(), #type_path::register_type_dependencies));
        }
        #[cfg(not(target_family = "wasm"))]
        #bevy_reflect_path::inventory::submit!(
            #bevy_reflect_path::DERIVED_REFLECT_TYPES(
                |reg: &mut #bevy_reflect_path::TypeRegistry| reg.register::<#type_path>()
            )
        );
    })
}
