mod bsn;

use proc_macro::TokenStream;

/// Creates a `Scene` using BSN (Bevy Scene Notation) syntax.
///
/// See [`bsn_list`] if you want to create multiple scenes at once,
/// or want to have multiple root entities.
///
/// See the `bevy_scene` crate docs for a high-level overview of how to use BSN.
#[proc_macro]
pub fn bsn(input: TokenStream) -> TokenStream {
    crate::bsn::bsn(input)
}

/// Creates a `SceneList` using BSN (Bevy Scene Notation) syntax.
///
/// This is useful when you want multiple root entities in your scene
/// that do not share a common parent, or if you want to create multiple scenes at once.
///
/// See [`bsn`] for more details on syntax.
/// See the `bevy_scene` crate docs for a high-level overview of how to use BSN.
#[proc_macro]
pub fn bsn_list(input: TokenStream) -> TokenStream {
    crate::bsn::bsn_list(input)
}
