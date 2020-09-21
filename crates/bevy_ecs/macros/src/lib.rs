extern crate proc_macro;

#[cfg(feature = "derive")]
mod derive;
use proc_macro::TokenStream;
use quote::quote;

/// Implement `Bundle` for a monomorphic struct
/// TODO: Support generic structs, e.g. for Camera3dComponents can be generic over projection type.
/// TODO: Make these structs !Component somehow (#392)
///
/// Using derived `Bundle` impls improves spawn performance and can be convenient when combined with
/// other derives like `serde::Deserialize`.
#[allow(clippy::cognitive_complexity)]
#[cfg(feature = "derive")]
#[proc_macro_derive(Bundle)]
pub fn derive_bundle(input: TokenStream) -> TokenStream {
    derive::derive_bundle(input)
}

/// Implements IntoForEachSystem and IntoQuerySystems for functions
#[proc_macro]
#[doc(hidden)]
pub fn impl_into_systems(_: TokenStream) -> TokenStream {
    let code = quote! {};
    TokenStream::from(code)
}
