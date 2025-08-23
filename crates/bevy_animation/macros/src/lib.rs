#![cfg_attr(docsrs, feature(doc_auto_cfg))]

//! Macros for deriving animation behaviors.

extern crate proc_macro;

mod animation_event;

use proc_macro::TokenStream;

/// Implements the `AnimationEvent` trait for a type - see the trait
/// docs for an example usage.
#[proc_macro_derive(AnimationEvent)]
pub fn derive_animation_event(input: TokenStream) -> TokenStream {
    animation_event::derive_animation_event(input)
}
