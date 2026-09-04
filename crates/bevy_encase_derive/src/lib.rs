#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]

use bevy_macro_utils::BevyManifest;
use encase_derive_impl::{implement, syn};
use quote::ToTokens;

const ENCASE: &str = "encase";

fn bevy_encase_path() -> syn::Path {
    // FIXME: `encase_derive_impl` still depends on `syn` 2, while `bevy_macro_utils`
    // uses `syn` 3, so the `syn::Path` types are incompatible. Round-trip through
    // a string until `encase` upgrades to `syn` 3, then build the `syn` 2 path directly again.
    let path_string = BevyManifest::shared(|bevy_manifest| {
        bevy_manifest
            .maybe_get_path("bevy_render")
            .map(|bevy_render_path| {
                format!(
                    "{} :: render_resource :: {ENCASE}",
                    bevy_render_path.to_token_stream()
                )
            })
            .unwrap_or_else(|| bevy_manifest.get_path(ENCASE).to_token_stream().to_string())
    });
    syn::parse_str(&path_string).expect("Failed to parse encase path")
}

implement!(bevy_encase_path());
