//! Proc macro for deriving labels for UI sub z layers

use bevy_macro_utils::{derive_label, BevyManifest};
use proc_macro::TokenStream;
use quote::format_ident;
use syn::{parse_macro_input, DeriveInput};

/// Proc macro for deriving labels for UI sub z layers
#[proc_macro_derive(GlobalZLayer)]
pub fn derive_ui_global_z_layer(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let mut trait_path = BevyManifest::shared(|manifest| manifest.get_path("bevy_ui"));
    trait_path.segments.push(format_ident!("ui_node").into());
    trait_path
        .segments
        .push(format_ident!("GlobalZLayer").into());
    derive_label(input, "GlobalZLayer", &trait_path)
}
