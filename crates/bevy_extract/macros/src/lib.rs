#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod extract_component;
mod extract_resource;

use bevy_macro_utils::BevyManifest;
// use proc_macro::TokenStream;

pub(crate) fn _bevy_extract_path() -> syn::Path {
    BevyManifest::shared(|manifest| manifest.get_path("bevy_extract"))
}
