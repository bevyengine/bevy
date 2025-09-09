#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]

use bevy_macro_utils::BevyManifest;
use encase_derive_impl::{implement, syn};

const ENCASE: &str = "encase";

fn bevy_encase_path() -> syn::Path {
    let bevy_manifest = BevyManifest::shared();
    bevy_manifest
        .maybe_get_path("bevy_render")
        .map(|bevy_render_path| {
            let mut segments = bevy_render_path.segments;
            segments.push(BevyManifest::parse_str("render_resource"));
            syn::Path {
                leading_colon: None,
                segments,
            }
        })
        .map(|path| {
            let mut segments = path.segments;
            segments.push(BevyManifest::parse_str(ENCASE));
            syn::Path {
                leading_colon: None,
                segments,
            }
        })
        .unwrap_or_else(|| bevy_manifest.get_path(ENCASE))
}

implement!(bevy_encase_path());
