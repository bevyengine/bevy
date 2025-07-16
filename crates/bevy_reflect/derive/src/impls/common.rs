use bevy_macro_utils::fq_std::{FQAny, FQOption, FQResult};

use quote::quote;

use crate::{derive_data::ReflectMeta, where_clause_options::WhereClauseOptions};

pub fn impl_full_reflect(where_clause_options: &WhereClauseOptions) -> proc_macro2::TokenStream {
    let meta = where_clause_options.meta();
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

#[cfg(feature = "auto_register")]
pub fn reflect_auto_registration(meta: &ReflectMeta) -> Option<proc_macro2::TokenStream> {
    if meta.attrs().no_auto_register() {
        return None;
    }

    let bevy_reflect_path = meta.bevy_reflect_path();
    let type_path = meta.type_path();

    if type_path.impl_is_generic() {
        return None;
    };

    #[cfg(feature = "auto_register_static")]
    {
        use std::{
            env, fs,
            io::Write,
            path::PathBuf,
            sync::{LazyLock, Mutex},
        };

        // Skip unless env var is set, otherwise this might slow down rust-analyzer
        if env::var("BEVY_REFLECT_AUTO_REGISTER_STATIC").is_err() {
            return None;
        }

        // Names of registrations functions will be stored in this file.
        // To allow writing to this file from multiple threads during compilation it is protected by mutex.
        // This static is valid for the duration of compilation of one crate and we have one file per crate,
        // so it is enough to protect compilation threads from overwriting each other.
        // This file is reset on every crate recompilation.
        //
        // It might make sense to replace the mutex with File::lock when file_lock feature becomes stable.
        static REGISTRATION_FNS_EXPORT: LazyLock<Mutex<fs::File>> = LazyLock::new(|| {
            let path = PathBuf::from("target").join("bevy_reflect_type_registrations");
            fs::DirBuilder::new()
                .recursive(true)
                .create(&path)
                .unwrap_or_else(|_| panic!("Failed to create {path:?}"));
            let file_path = path.join(env::var("CARGO_CRATE_NAME").unwrap());
            let file = fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&file_path)
                .unwrap_or_else(|_| panic!("Failed to create {file_path:?}"));
            Mutex::new(file)
        });

        let export_name = format!("_bevy_reflect_register_{}", uuid::Uuid::new_v4().as_u128());

        {
            let mut file = REGISTRATION_FNS_EXPORT.lock().unwrap();
            writeln!(file, "{export_name}")
                .unwrap_or_else(|_| panic!("Failed to write registration function {export_name}"));
            // We must sync_data to ensure all content is written before releasing the lock.
            file.sync_data().unwrap();
        };

        Some(quote! {
            /// # Safety
            /// This function must only be used by the `load_type_registrations` macro.
            #[unsafe(export_name=#export_name)]
            pub unsafe extern "Rust" fn bevy_register_type(registry: &mut #bevy_reflect_path::TypeRegistry) {
                <#type_path as #bevy_reflect_path::__macro_exports::RegisterForReflection>::__register(registry);
            }
        })
    }

    #[cfg(all(
        feature = "auto_register_inventory",
        not(feature = "auto_register_static")
    ))]
    {
        Some(quote! {
            #bevy_reflect_path::__macro_exports::auto_register::inventory::submit!{
                #bevy_reflect_path::__macro_exports::auto_register::AutomaticReflectRegistrations(
                    <#type_path as #bevy_reflect_path::__macro_exports::auto_register::RegisterForReflection>::__register
                )
            }
        })
    }
}
