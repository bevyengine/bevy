extern crate proc_macro;

use bevy_macro_utils::{derive_label, BevyManifest};
use proc_macro::TokenStream;
use quote::format_ident;

#[proc_macro_derive(SubAppLabel)]
pub fn derive_sub_app_label(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let mut trait_path = bevy_app_path();
    trait_path
        .segments
        .push(format_ident!("SubAppLabel").into());
    derive_label(input, trait_path)
}

fn bevy_app_path() -> syn::Path {
    BevyManifest::default().get_path("bevy_app")
}
