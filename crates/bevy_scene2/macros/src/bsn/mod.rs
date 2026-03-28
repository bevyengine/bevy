use crate::bsn::{
    codegen::{BsnCodegenCtx, EntityRefs},
    traits::BsnTokenStream,
    types::{BsnListRoot, BsnRoot},
};
use bevy_macro_utils::BevyManifest;
use proc_macro::TokenStream;
use syn::parse_macro_input;

pub mod codegen;
pub mod parse;
pub mod traits;
pub mod types;

pub fn bsn(input: TokenStream) -> TokenStream {
    bsn_token_stream::<BsnRoot>(input)
}

pub fn bsn_list(input: TokenStream) -> TokenStream {
    bsn_token_stream::<BsnListRoot>(input)
}

fn bsn_token_stream<T: BsnTokenStream>(input: TokenStream) -> TokenStream {
    let scene = parse_macro_input!(input as T);
    let (bevy_scene, bevy_ecs, _bevy_asset) = BevyManifest::shared(|manifest| {
        (
            manifest.get_path("bevy_scene2"),
            manifest.get_path("bevy_ecs"),
            manifest.get_path("bevy_asset"),
        )
    });
    let mut entity_refs = EntityRefs::default();
    let mut ctx = BsnCodegenCtx {
        bevy_scene: &bevy_scene,
        bevy_ecs: &bevy_ecs,
        entity_refs: &mut entity_refs,
        errors: Vec::new(),
    };

    TokenStream::from(scene.to_tokens(&mut ctx))
}
