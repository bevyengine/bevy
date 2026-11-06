#![expect(missing_docs, reason = "Work in progress")]

mod reflect;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

/// Derives the `Reflect` and `Typed` trait for the given type.
#[proc_macro_derive(Reflect, attributes(reflect))]
pub fn derive_reflect(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    match reflect::derive_reflect(ast) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

/// Implements reflection for a type defined in another crate. The type is either described by a
/// re-statement of its definition, or reflected as a primitive (`Kind::Primitive`).
///
/// # Re-stated definitions
///
/// A struct or enum definition generates the same impls as `#[derive(Reflect)]`, except automatic
/// registration. The re-statement must list every field (or variant) of the type with the correct
/// types, which is verified at compile time. Fields must be visible to the calling crate.
///
/// The `#[type_path = "path::to::module"]` attribute is required, and specifies the module that
/// contains the type.
///
/// ```ignore
/// impl_reflect! {
///     #[reflect(Default)]
///     #[type_path = "core::option"]
///     enum Option<T> {
///         None,
///         Some(T),
///     }
/// }
/// ```
///
/// # Primitives
///
/// Any other type is reflected as a primitive, whose internals are not reflected. The type is
/// specified using its full path (starting with the crate name), and generic parameters can declare
/// bounds. Types without a module (ex: `u32`) can be specified by name. Types whose path can't be
/// expressed this way (ex: references) specify their full type path using
/// `#[full_type_path = "path::to::Type"]`.
///
/// Traits and type data are registered with `#[reflect(...)]`, like re-stated definitions. The
/// `Primitive` variant is selected with `#[reflect(primitive = Variant)]`, and defaults to
/// [`Primitive::Opaque`](https://docs.rs/bevy_reflect/latest/bevy_reflect/enum.Primitive.html).
///
/// ```ignore
/// impl_reflect!(
///     #[reflect(Clone, Debug, Hash, PartialEq, PartialOrd, Default)]
///     ::core::time::Duration
/// );
/// impl_reflect!(#[reflect(Clone)] ::core::ops::Range<T: Clone>);
/// impl_reflect!(
///     #[reflect(primitive = Int, Clone, Debug, Hash, PartialEq, PartialOrd, Default)]
///     u32
/// );
/// impl_reflect!(
///     #[reflect(Clone, Debug)]
///     #[full_type_path = "core::panic::Location"]
///     &'static Location<'static>
/// );
/// ```
#[proc_macro]
pub fn impl_reflect(input: TokenStream) -> TokenStream {
    match reflect::impl_reflect(input.into()) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

/// Collects and loads type registrations when using `auto_register_static` feature.
///
/// Correctly using this macro requires following:
/// 1. This macro must be called **last** during compilation. This can be achieved by putting your main function
///    in a separate crate or restructuring your project to be separated into `bin` and `lib`, and putting this macro in `bin`.
///    Any automatic type registrations using `#[derive(Reflect)]` within the same crate as this macro are not guaranteed to run.
/// 2. Your project must be compiled with `auto_register_static` feature **and** `BEVY_REFLECT_AUTO_REGISTER_STATIC=1` env variable.
///    Enabling the feature generates registration functions while setting the variable enables export and
///    caching of registration function names.
/// 3. Must be called before using `TypeRegistry::register_derived_types`.
///
/// If you're experiencing linking issues try running `cargo clean` before rebuilding.
#[proc_macro]
pub fn load_type_registrations(_input: TokenStream) -> TokenStream {
    #[cfg(feature = "auto_register_static")]
    {
        let bevy_reflect =
            bevy_macro_utils::BevyManifest::shared(|manifest| manifest.get_path("bevy_reflect2"));
        reflect::load_type_registrations(&bevy_reflect).into()
    }
    #[cfg(not(feature = "auto_register_static"))]
    TokenStream::new()
}
