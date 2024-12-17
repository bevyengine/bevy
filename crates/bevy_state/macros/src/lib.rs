#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

extern crate proc_macro;

mod states;

use bevy_macro_utils::BevyManifest;
use proc_macro::TokenStream;

#[proc_macro_derive(States, attributes(states))]
pub fn derive_states(input: TokenStream) -> TokenStream {
    states::derive_states(input)
}

#[proc_macro_derive(SubStates, attributes(states, source))]
pub fn derive_substates(input: TokenStream) -> TokenStream {
    states::derive_substates(input)
}

pub(crate) fn bevy_state_path() -> syn::Path {
    BevyManifest::shared().get_path("bevy_state")
}
