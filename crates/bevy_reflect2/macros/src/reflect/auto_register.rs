use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Path};

use crate::reflect::attributes::ContainerAttributes;

/// Directory (relative to the workspace root) where `auto_register_static` stores the names of
/// generated registration functions, one file per crate.
#[cfg(feature = "auto_register_static")]
pub(crate) const REGISTRATIONS_DIR: &str = "bevy_reflect_type_registrations";

/// Generates the code that automatically registers `Self` when calling
/// `TypeRegistry::register_derived_types`.
///
/// Generic types are skipped, as their desired monomorphized representations must be registered manually.
pub(crate) fn auto_registration(
    ast: &DeriveInput,
    bevy_reflect: &Path,
    container_attributes: &ContainerAttributes,
) -> TokenStream {
    if container_attributes.no_auto_register || !ast.generics.params.is_empty() {
        return quote! {};
    }

    let type_ident = &ast.ident;

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
            return quote! {};
        }

        // Names of registrations functions will be stored in this file.
        // To allow writing to this file from multiple threads during compilation it is protected by mutex.
        // This static is valid for the duration of compilation of one crate and we have one file per crate,
        // so it is enough to protect compilation threads from overwriting each other.
        // This file is reset on every crate recompilation.
        //
        // It might make sense to replace the mutex with File::lock when file_lock feature becomes stable.
        static REGISTRATION_FNS_EXPORT: LazyLock<Mutex<fs::File>> = LazyLock::new(|| {
            let path = PathBuf::from("target").join(REGISTRATIONS_DIR);
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

        quote! {
            const _: () = {
                /// # Safety
                /// This function must only be used by the `load_type_registrations` macro.
                #[unsafe(export_name = #export_name)]
                pub unsafe extern "Rust" fn bevy_register_type(registry: &mut #bevy_reflect::TypeRegistry) {
                    registry.register::<#type_ident>();
                }
            };
        }
    }

    #[cfg(all(
        feature = "auto_register_inventory",
        not(feature = "auto_register_static")
    ))]
    {
        quote! {
            #bevy_reflect::__macro_exports::auto_register::inventory::submit! {
                #bevy_reflect::__macro_exports::auto_register::AutomaticReflectRegistrations(
                    #bevy_reflect::TypeRegistry::register::<#type_ident>
                )
            }
        }
    }

    #[cfg(not(any(feature = "auto_register_inventory", feature = "auto_register_static")))]
    {
        let _ = (type_ident, bevy_reflect);
        quote! {}
    }
}

/// Generates the code that loads all registration functions collected by `auto_register_static`.
#[cfg(feature = "auto_register_static")]
pub(crate) fn load_type_registrations(bevy_reflect: &Path) -> TokenStream {
    use std::{fs, io::Read, path::PathBuf};

    let Ok(dir) = fs::read_dir(PathBuf::from("target").join(REGISTRATIONS_DIR)) else {
        return TokenStream::new();
    };
    let mut str_buf = String::new();
    let mut registration_fns = Vec::new();
    for file_path in dir {
        let mut file = fs::OpenOptions::new()
            .read(true)
            .open(file_path.unwrap().path())
            .unwrap();
        file.read_to_string(&mut str_buf).unwrap();
        registration_fns.extend(
            str_buf
                .lines()
                .filter(|s| !s.is_empty())
                .map(|s| s.parse::<TokenStream>().expect("Unexpected function name")),
        );
        str_buf.clear();
    }
    quote! {
        {
            fn _register_types() {
                unsafe extern "Rust" {
                    #( safe fn #registration_fns(registry_ptr: &mut #bevy_reflect::TypeRegistry); )*
                };
                #( #bevy_reflect::__macro_exports::auto_register::push_registration_fn(#registration_fns); )*
            }
            _register_types();
        }
    }
}
