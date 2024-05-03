// FIXME(3492): remove once docs are ready
#![allow(missing_docs)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

extern crate proc_macro;

mod states;

use bevy_macro_utils::{derive_label, ensure_no_collision, get_struct_fields, BevyManifest};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, parse_quote, punctuated::Punctuated, spanned::Spanned, token::Comma,
    ConstParam, DeriveInput, GenericParam, Ident, Index, TypeParam,
};

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
