use crate::bsn::{
    codegen::EntityRefs,
    types::{BsnListRoot, BsnRoot},
};
use bevy_macro_utils::BevyManifest;
use proc_macro::TokenStream;
use syn::parse_macro_input;

pub mod codegen;
pub mod parse;
pub mod types;

pub fn bsn(input: TokenStream) -> TokenStream {
    let scene = parse_macro_input!(input as BsnRoot);
    let (bevy_scene, bevy_ecs, bevy_asset) = BevyManifest::shared(|manifest| {
        (
            manifest.get_path("bevy_scene2"),
            manifest.get_path("bevy_ecs"),
            manifest.get_path("bevy_asset"),
        )
    });
    let mut entity_refs = EntityRefs::default();
    TokenStream::from(scene.to_tokens(&bevy_scene, &bevy_ecs, &bevy_asset, &mut entity_refs))
}

pub fn bsn_list(input: TokenStream) -> TokenStream {
    let scene = parse_macro_input!(input as BsnListRoot);
    let (bevy_scene, bevy_ecs, bevy_asset) = BevyManifest::shared(|manifest| {
        (
            manifest.get_path("bevy_scene2"),
            manifest.get_path("bevy_ecs"),
            manifest.get_path("bevy_asset"),
        )
    });
    let mut entity_refs = EntityRefs::default();
    TokenStream::from(scene.to_tokens(&bevy_scene, &bevy_ecs, &bevy_asset, &mut entity_refs))
}
