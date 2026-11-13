#![allow(private_interfaces)]

//! Fuzz harness for the `bsn!` and `bsn_list!` proc-macros.
//!
//! The macro crate is `proc-macro = true` and cannot be linked into a fuzz
//! binary, so this crate re-includes the BSN parser/codegen modules directly
//! via `#[path]` and exposes a wrapper.

#[path = "bsn"]
pub mod _bsn {
    #[path = "../../../src/bsn/codegen.rs"]
    pub mod codegen;
    #[path = "../../../src/bsn/parse.rs"]
    pub mod parse;
    #[path = "../../../src/bsn/types.rs"]
    pub mod types;
}

use _bsn::codegen::{BsnCodegenCtx, BsnTokenStream, EntityRefs, HoistedExpressions};
use proc_macro2::TokenStream;
use syn::parse::Parse;

pub fn try_codegen<T: Parse + BsnTokenStream>(input: TokenStream) -> syn::Result<TokenStream> {
    let scene = syn::parse2::<T>(input)?;

    let bevy_scene: syn::Path = syn::parse_quote!(::bevy_scene);
    let bevy_ecs: syn::Path = syn::parse_quote!(::bevy_ecs);
    let mut entity_refs = EntityRefs::default();
    let mut hoisted_expressions = HoistedExpressions::default();

    let mut ctx = BsnCodegenCtx {
        bevy_scene: &bevy_scene,
        bevy_ecs: &bevy_ecs,
        entity_refs: &mut entity_refs,
        invocation_index: syn::parse_quote!(("", 0, 0)),
        hoisted_expressions: &mut hoisted_expressions,
        errors: Vec::new(),
        deprecations: Vec::new(),
    };

    Ok(scene.into_tokens(&mut ctx))
}

/// Maximum nesting of `(`, `[`, `{` and `<` accepted by [`too_deep`].
const MAX_NESTING: usize = 128;

pub fn too_deep(data: &str) -> bool {
    let mut depth = 0usize;
    for c in data.chars() {
        match c {
            '(' | '[' | '{' | '<' => {
                depth += 1;
                if depth > MAX_NESTING {
                    return true;
                }
            }
            ')' | ']' | '}' | '>' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    false
}
