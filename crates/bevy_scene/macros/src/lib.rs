mod bsn;
mod scene_component;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro]
pub fn bsn(input: TokenStream) -> TokenStream {
    crate::bsn::bsn(input)
}

#[proc_macro]
pub fn bsn_list(input: TokenStream) -> TokenStream {
    crate::bsn::bsn_list(input)
}

#[proc_macro_derive(
    SceneComponent,
    attributes(component, require, relationship, relationship_target, entities, scene)
)]
pub fn derive_scene_component(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    TokenStream::from(scene_component::derive_scene_component(&mut ast))
}
