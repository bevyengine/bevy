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

/// Implement Bundle and DynamicBundle for tuples of length up to `k`
/// Used in bevy_hecs
#[proc_macro]
#[doc(hidden)]
pub fn impl_tuple_bundle(_: TokenStream) -> TokenStream {
    let code = quote! {};
    TokenStream::from(code)
}

/// Implement the Query trait for tuples of length up to `k` and `Or`s of those tuples
/// Also implement the `FetchResources` trait for tuples of length up to `k` and `OrRes`s of those tuples
// TODO: Support implementing the Query trait for custom types?
#[doc(hidden)]
#[proc_macro]
pub fn impl_tuple_queries_resources(_: TokenStream) -> TokenStream {
    let code = quote! {};
    TokenStream::from(code)
}

/// Implement the `FetchResources` trait for tuples of length up to `k` and `Or`s of those tuples
#[proc_macro]
#[doc(hidden)]
pub fn impl_tuple_fetch_resources(_: TokenStream) -> TokenStream {
    let code = quote! {};
    TokenStream::from(code)
}

/// Implements IntoForEachSystem and IntoQuerySystems for functions
#[proc_macro]
#[doc(hidden)]
pub fn impl_into_system(_: TokenStream) -> TokenStream {
    let code = quote! {};
    TokenStream::from(code)
}
