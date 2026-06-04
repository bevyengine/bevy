pub mod codegen;
pub mod parse;
pub mod types;

use codegen::*;
use types::*;

use bevy_macro_utils::BevyManifest;
use proc_macro::{Span, TokenStream};
use syn::{parse_macro_input, parse_quote};

pub fn bsn(input: TokenStream) -> TokenStream {
    bsn_token_stream::<BsnRoot>(input)
}

pub fn bsn_list(input: TokenStream) -> TokenStream {
    bsn_token_stream::<BsnListRoot>(input)
}

fn bsn_token_stream<T: BsnTokenStream>(input: TokenStream) -> TokenStream {
    let scene = parse_macro_input!(input as T);

    let (bevy_scene, bevy_ecs) = BevyManifest::shared(|manifest| {
        (
            manifest.get_path("bevy_scene"),
            manifest.get_path("bevy_ecs"),
        )
    });
    let mut entity_refs = EntityRefs::default();
    let mut hoisted_expressions = HoistedExpressions::default();
    let call_site = Span::call_site();
    let file = call_site.file();
    let line = call_site.line();
    let column = call_site.column();
    let mut ctx = BsnCodegenCtx {
        bevy_scene: &bevy_scene,
        bevy_ecs: &bevy_ecs,
        entity_refs: &mut entity_refs,
        invocation_index: parse_quote!((#file, #line, #column)),
        hoisted_expressions: &mut hoisted_expressions,
        errors: Vec::new(),
    };

    TokenStream::from(scene.to_tokens(&mut ctx))
}
