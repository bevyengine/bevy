use crate::bsn::types::{BsnRoot, BsnSceneListItems};
use bevy_macro_utils::BevyManifest;
use proc_macro::TokenStream;
use syn::parse_macro_input;

pub mod codegen;
pub mod parse;
pub mod types;

pub fn bsn(input: TokenStream) -> TokenStream {
    let scene = parse_macro_input!(input as BsnRoot);
    let manifest = BevyManifest::shared();
    let bevy_scene = manifest.get_path("bevy_scene2");
    let bevy_ecs = manifest.get_path("bevy_ecs");
    let bevy_asset = manifest.get_path("bevy_asset");
    TokenStream::from(scene.to_tokens(&bevy_scene, &bevy_ecs, &bevy_asset))
}

pub fn bsn_list(input: TokenStream) -> TokenStream {
    let scene = parse_macro_input!(input as BsnSceneListItems);
    let manifest = BevyManifest::shared();
    let bevy_scene = manifest.get_path("bevy_scene2");
    let bevy_ecs = manifest.get_path("bevy_ecs");
    let bevy_asset = manifest.get_path("bevy_asset");
    TokenStream::from(scene.to_tokens(&bevy_scene, &bevy_ecs, &bevy_asset))
}
