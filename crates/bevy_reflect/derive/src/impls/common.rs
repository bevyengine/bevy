use bevy_macro_utils::fq_std::{FQAny, FQOption, FQResult};

use quote::quote;

use crate::{derive_data::ReflectMeta, where_clause_options::WhereClauseOptions};

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
            fn into_any(self: #bevy_reflect_path::__macro_exports::alloc_utils::Box<Self>) -> #bevy_reflect_path::__macro_exports::alloc_utils::Box<dyn #FQAny> {
                #bevy_reflect_path::__macro_exports::alloc_utils::Box::new(self.0)
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
            fn into_any(self: #bevy_reflect_path::__macro_exports::alloc_utils::Box<Self>) -> #bevy_reflect_path::__macro_exports::alloc_utils::Box<dyn #FQAny> {
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
            fn into_reflect(self: #bevy_reflect_path::__macro_exports::alloc_utils::Box<Self>) -> #bevy_reflect_path::__macro_exports::alloc_utils::Box<dyn #bevy_reflect_path::Reflect> {
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
                value: #bevy_reflect_path::__macro_exports::alloc_utils::Box<dyn #bevy_reflect_path::Reflect>
            ) -> #FQResult<(), #bevy_reflect_path::__macro_exports::alloc_utils::Box<dyn #bevy_reflect_path::Reflect>> {
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
            self: #bevy_reflect_path::__macro_exports::alloc_utils::Box<Self>
        ) -> #FQResult<#bevy_reflect_path::__macro_exports::alloc_utils::Box<dyn #bevy_reflect_path::Reflect>, #bevy_reflect_path::__macro_exports::alloc_utils::Box<dyn #bevy_reflect_path::PartialReflect>> {
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
        fn into_partial_reflect(self: #bevy_reflect_path::__macro_exports::alloc_utils::Box<Self>) -> #bevy_reflect_path::__macro_exports::alloc_utils::Box<dyn #bevy_reflect_path::PartialReflect> {
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
