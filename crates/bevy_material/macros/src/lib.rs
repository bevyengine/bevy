#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![expect(unused, reason = "Setup.")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

use bevy_macro_utils::BevyManifest;

pub(crate) fn bevy_material_path() -> syn::Path {
    BevyManifest::shared(|manifest| manifest.get_path("bevy_material"))
}
