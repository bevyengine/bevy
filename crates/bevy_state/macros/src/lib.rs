#![cfg_attr(docsrs, feature(doc_auto_cfg))]

//! Macros for deriving `States` and `SubStates` traits.

extern crate proc_macro;

mod states;

use bevy_macro_utils::BevyManifest;
use proc_macro::TokenStream;

/// Implements the `States` trait for a type - see the trait
/// docs for an example usage.
#[proc_macro_derive(States, attributes(states))]
pub fn derive_states(input: TokenStream) -> TokenStream {
    states::derive_states(input)
}

/// Implements the `SubStates` trait for a type - see the trait
/// docs for an example usage.
#[proc_macro_derive(SubStates, attributes(states, source))]
pub fn derive_substates(input: TokenStream) -> TokenStream {
    states::derive_substates(input)
}

pub(crate) fn bevy_state_path() -> syn::Path {
    BevyManifest::shared().get_path("bevy_state")
}
