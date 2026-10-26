//! Proc macro for deriving labels for UI paint layers

use bevy_macro_utils::{derive_label, BevyManifest};
use proc_macro::TokenStream;
use quote::format_ident;
use syn::{parse_macro_input, DeriveInput};

/// Proc macro for deriving labels for UI paint layers
#[proc_macro_derive(PaintLayer)]
pub fn derive_paint_layer(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let mut trait_path = BevyManifest::shared(|manifest| manifest.get_path("bevy_ui_render"));
    trait_path
        .segments
        .push(format_ident!("paint_layers").into());
    trait_path.segments.push(format_ident!("PaintLayer").into());
    derive_label(input, "PaintLayer", &trait_path)
}
