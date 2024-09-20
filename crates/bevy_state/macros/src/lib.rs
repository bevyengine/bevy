// FIXME(15321): solve CI failures, then replace with `#![expect()]`.
#![allow(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

extern crate proc_macro;

mod states;

use bevy_macro_utils::BevyManifest;
use proc_macro::TokenStream;

#[proc_macro_derive(States)]
pub fn derive_states(input: TokenStream) -> TokenStream {
    states::derive_states(input)
}

#[proc_macro_derive(SubStates, attributes(source))]
pub fn derive_substates(input: TokenStream) -> TokenStream {
    states::derive_substates(input)
}

pub(crate) fn bevy_state_path() -> syn::Path {
    BevyManifest::default().get_path("bevy_state")
}
